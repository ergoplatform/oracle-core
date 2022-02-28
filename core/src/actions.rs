/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::node_interface::{
    address_to_raw_for_register, address_to_tree, current_block_height,
    raw_from_register_to_address, send_transaction, serialize_boxes,
    serialized_unspent_boxes_with_min_total,
};
use crate::oracle_config::PoolParameters;
use crate::oracle_state::{LiveEpochState, OraclePool};
use crate::templates::BASIC_TRANSACTION_SEND_REQUEST;
use crate::Result;
use ergo_lib::chain::ergo_box::ErgoBox;
use ergo_lib::chain::Base16Str;
use ergo_lib::ergotree_ir::mir::constant::Constant;
use ergo_offchain_utilities::encoding::{
    serialize_hex_encoded_string, string_to_blake2b_hash, unwrap_hex_encoded_string, unwrap_long,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CollectionError {
    #[error("Failed collecting datapoints. The minimum consensus number could not be reached, meaning that an insufficient number of oracles posted datapoints within the deviation range.")]
    FailedToReachConsensus(),
    #[error("Failed collecting datapoints. The local oracle did not post a datapoint in the current epoch.")]
    LocalOracleFailedToPostDatapoint(),
    #[error("Failed collecting datapoints. The local oracle did not post a datapoint within the deviation range (when compared to datapoints posted by other oracles in the pool).")]
    LocalOracleFailedToPostDatapointWithinDeviation(),
}

impl OraclePool {
    /// Generates and submits the "Commit Datapoint" action tx
    pub fn action_commit_datapoint(&self, datapoint: u64) -> Result<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Defining the registers of the output box
        let live_epoch_id = self.get_live_epoch_state()?.epoch_id;
        let registers = object! {
            "R4": address_to_raw_for_register(&self.local_oracle_address)?,
            "R5": serialize_hex_encoded_string(&live_epoch_id)?.base16_str(),
            "R6": Constant::from(datapoint as i64).base16_str(),
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_participant_token.to_string(),
            "amount": 1
        };

        let mut inputs_raw = vec![self.local_oracle_datapoint_scan.get_serialized_box()?];
        inputs_raw.append(&mut serialized_unspent_boxes_with_min_total(
            parameters.base_fee,
        )?);

        // Filling out the json tx request template
        req["requests"][0]["address"] = self.datapoint_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers;
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = inputs_raw.into();
        req["dataInputsRaw"] = vec![self.live_epoch_stage.get_serialized_box()?].into();
        req["fee"] = parameters.base_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }

    /// Generates and submits the "Collect Funds" action tx
    pub fn action_collect_funds(&self) -> Result<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": Constant::from(epoch_prep_state.latest_pool_datapoint as i64).base16_str(),

            "R5": Constant::from(epoch_prep_state.next_epoch_ends as i32).base16_str(),
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_nft.to_string(),
            "amount": 1
        };

        // Create input boxes Vec with serialized Epoch Preparation box inside
        let mut unserialized_input_boxes = vec![self.epoch_preparation_stage.get_box()?];
        // Acquire all Pool Deposit boxes
        let mut initial_deposit_boxes = self.pool_deposit_stage.get_boxes()?;
        // Only append up to 27 boxes for now. This is to prevent exceeding execution limit for txs.
        if initial_deposit_boxes.len() > 27 {
            unserialized_input_boxes.append(&mut initial_deposit_boxes[..27].to_vec());
        } else {
            unserialized_input_boxes.append(&mut initial_deposit_boxes);
        }

        // Define the fee for the current action
        let action_fee = 500000 * unserialized_input_boxes.len() as u64;

        // Serialize boxes and add extra box for paying fee
        let mut serialized_input_boxes = serialize_boxes(&unserialized_input_boxes)?;
        serialized_input_boxes.append(&mut serialized_unspent_boxes_with_min_total(
            action_fee,
        )?);

        // Sum up the new total minus tx fee
        let total_input_ergs = unserialized_input_boxes
            .iter()
            .fold(0, |acc, b| acc + b.value.as_u64());

        // Filling out the json tx request template
        req["requests"][0]["value"] = total_input_ergs.into();
        req["requests"][0]["address"] =
            self.epoch_preparation_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers;
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = serialized_input_boxes.into();
        req["fee"] = action_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }

    /// Generates and submits the "Start Next Epoch" action tx
    pub fn action_start_next_epoch(&self) -> Result<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": Constant::from(epoch_prep_state.latest_pool_datapoint as i64).base16_str(),
            "R5": Constant::from(epoch_prep_state.next_epoch_ends as i32).base16_str(),
            "R6": serialize_hex_encoded_string(&string_to_blake2b_hash(address_to_tree(&self.epoch_preparation_stage.contract_address)?)?)?.base16_str(),
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_nft.to_string(),
            "amount": 1
        };

        let mut inputs_raw = vec![self.epoch_preparation_stage.get_serialized_box()?];
        inputs_raw.append(&mut serialized_unspent_boxes_with_min_total(
            parameters.base_fee,
        )?);

        // Filling out the json tx request template
        req["requests"][0]["value"] = epoch_prep_state.funds.into();
        req["requests"][0]["address"] = self.live_epoch_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers;
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = inputs_raw.into();
        req["fee"] = parameters.base_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }

    /// Generates and submits the "Create New Epoch" action tx
    pub fn action_create_new_epoch(&self) -> Result<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Define the new epoch finish height based off of current height
        let new_finish_height = current_block_height()?
            + parameters.epoch_preparation_length
            + parameters.live_epoch_length
            + parameters.buffer_length;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": Constant::from(epoch_prep_state.latest_pool_datapoint as i64).base16_str(),
            "R5": Constant::from(new_finish_height as i32).base16_str(),
            "R6": serialize_hex_encoded_string(&string_to_blake2b_hash(address_to_tree(&self.epoch_preparation_stage.contract_address)?)?)?.base16_str(),
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_nft.to_string(),
            "amount": 1
        };

        let mut inputs_raw = vec![self.epoch_preparation_stage.get_serialized_box()?];
        inputs_raw.append(&mut serialized_unspent_boxes_with_min_total(
            parameters.base_fee,
        )?);

        // Filling out the json tx request template
        req["requests"][0]["value"] = epoch_prep_state.funds.into();
        req["requests"][0]["address"] = self.live_epoch_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers;
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = inputs_raw.into();
        req["fee"] = parameters.base_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }

    /// Generates and submits the "Collect Datapoints" action tx
    pub fn action_collect_datapoints(&self) -> Result<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        let live_epoch_state = self.get_live_epoch_state()?;

        // Filter out Datapoint boxes not from the latest epoch
        let current_epoch_datapoint_boxes =
            current_epoch_boxes_filter(&self.datapoint_stage.get_boxes()?, &live_epoch_state);
        // Sort Datapoint boxes in decreasing order
        let sorted_datapoint_boxes = sort_datapoint_boxes(&current_epoch_datapoint_boxes);

        // Acquire the finalized oracle pool datapoint and the list of successful datapoint boxes which were within the deviation range
        let (finalized_datapoint, successful_boxes) = finalize_datapoint(
            &sorted_datapoint_boxes,
            parameters.deviation_range as i64, // Make sure to change this to config #
            parameters.consensus_num as i64,   // Make sure to change this to config #
        )?;

        // Find the index of the local oracle's Datapoint box in the successful boxes list
        let local_datapoint_box_index = find_box_index_in_list(
            self.local_oracle_datapoint_scan.get_box()?,
            &successful_boxes,
        )
        .ok_or(CollectionError::LocalOracleFailedToPostDatapointWithinDeviation())?;

        // Tx fee for the transaction
        let tx_fee = (parameters.base_fee) * sorted_datapoint_boxes.len() as u64;
        // Define the new value of the oracle pool box after payouts/tx fee
        let new_box_value = live_epoch_state.funds
            - (parameters.oracle_payout_price * (successful_boxes.len() as u64 + 1));
        // Define the finish height of the following epoch
        let new_finish_height = self.get_live_epoch_state()?.epoch_ends
            + parameters.epoch_preparation_length
            + parameters.live_epoch_length;

        // Defining json request for the oracle pool box
        let token_json = object! {
            "tokenId": self.oracle_pool_nft.to_string(),
            "amount": 1
        };
        let registers = object! {
            "R4": Constant::from(finalized_datapoint as i64).base16_str(),
            "R5": Constant::from(new_finish_height as i32).base16_str(),
        };
        let mut inputs_raw = vec![self.live_epoch_stage.get_serialized_box()?];
        inputs_raw.append(&mut serialized_unspent_boxes_with_min_total(tx_fee)?);

        req["requests"][0]["value"] = new_box_value.into();
        req["requests"][0]["address"] =
            self.epoch_preparation_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers;
        req["requests"][0]["assets"] = vec![token_json].into();

        // Filling out requests for the oracle payout outputs
        for b in &successful_boxes {
            // Get the P2PK from the hex encoded constant string minus the first two characters which are a register type descriptor
            let oracle_address = raw_from_register_to_address(
                &b.additional_registers.get_ordered_values()[0].base16_str(),
            )?;
            req["requests"]
                .push(object! {
                    "address": oracle_address,
                    "value": parameters.oracle_payout_price,
                })
                .ok();
        }
        // Add the local oracle Datapoint box index into R4 of the first oracle payout box
        req["requests"][1]["registers"] = object! {
            "R4": Constant::from(local_datapoint_box_index as i32).base16_str()
        };
        // Pay the local oracle double due to being Collector
        req["requests"][local_datapoint_box_index + 1]["value"] =
            (parameters.oracle_payout_price * 2).into();
        // Filling out the rest of the json request
        req["inputsRaw"] = inputs_raw.into();
        req["dataInputsRaw"] = serialize_boxes(&successful_boxes)?.into();
        req["fee"] = tx_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }
}

/// Given an `ErgoBox`, find its index in the input `Vec<ErgoBox>`
/// If index cannot be found, then local oracle has not submit their
/// own datapoint, and thus the function returns `None`
fn find_box_index_in_list(
    search_box: ErgoBox,
    sorted_datapoint_boxes: &Vec<ErgoBox>,
) -> Option<usize> {
    sorted_datapoint_boxes
        .iter()
        .position(|b| b.clone() == search_box)
}

/// Removes boxes which do not have a valid datapoint Long in R6.
pub fn valid_boxes_filter(boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
    let mut valid_boxes = vec![];
    for b in boxes {
        if unwrap_long(&b.additional_registers.get_ordered_values()[2]).is_ok() {
            valid_boxes.push(b.clone());
        }
    }
    valid_boxes
}

/// Filters out Datapoint boxes that are not from the current epoch
/// Also calls `valid_boxes_filter()` to remove invalid boxes.
pub fn current_epoch_boxes_filter(
    datapoint_boxes: &Vec<ErgoBox>,
    live_epoch_state: &LiveEpochState,
) -> Vec<ErgoBox> {
    let mut filtered_boxes = vec![];
    let valid_boxes = valid_boxes_filter(datapoint_boxes);
    for b in valid_boxes {
        if let Ok(s) = unwrap_hex_encoded_string(&b.additional_registers.get_ordered_values()[1]) {
            if s == live_epoch_state.epoch_id {
                filtered_boxes.push(b.clone());
            }
        }
    }
    filtered_boxes
}

/// Sort Datapoint boxes in decreasing order (from highest to lowest) based on Datapoint value.
pub fn sort_datapoint_boxes(boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
    let mut datapoint_boxes = boxes.clone();
    datapoint_boxes
        .sort_by_key(|b| unwrap_long(&b.additional_registers.get_ordered_values()[2]).unwrap_or(0));
    datapoint_boxes.reverse();
    datapoint_boxes
}

/// Function for averaging datapoints from a list of Datapoint boxes.
pub fn average_datapoints(boxes: &Vec<ErgoBox>) -> Result<u64> {
    let datapoints_sum = boxes.iter().fold(Ok(0), |acc: Result<i64>, b| {
        Ok(acc? + unwrap_long(&b.additional_registers.get_ordered_values()[2])?)
    })?;
    if boxes.is_empty() {
        return Err(CollectionError::LocalOracleFailedToPostDatapoint().into());
    }
    let average = datapoints_sum / boxes.len() as i64;
    Ok(average as u64)
}

/// Verifies that the list of sorted Datapoint boxes passes the deviation check
pub fn deviation_check(deviation_range: i64, datapoint_boxes: &Vec<ErgoBox>) -> Result<bool> {
    let num = datapoint_boxes.len();
    let max_datapoint =
        unwrap_long(&datapoint_boxes[0].additional_registers.get_ordered_values()[2])?;
    let min_datapoint = unwrap_long(
        &datapoint_boxes[num - 1]
            .additional_registers
            .get_ordered_values()[2],
    )?;
    let deviation_delta = max_datapoint * deviation_range / 100;

    Ok(min_datapoint >= max_datapoint - deviation_delta)
}

/// Finds whether the first or the last value in a list of sorted Datapoint boxes
/// deviates more compared to their adjacted datapoint, and then removes
/// said datapoint which deviates further.
pub fn remove_largest_local_deviation_datapoint(
    datapoint_boxes: &Vec<ErgoBox>,
) -> Result<Vec<ErgoBox>> {
    let mut processed_boxes = datapoint_boxes.clone();

    // Check if sufficient number of datapoint boxes to start removing
    if datapoint_boxes.len() <= 2 {
        Err(CollectionError::FailedToReachConsensus().into())
    } else {
        // Deserialize all the datapoints in a list
        let dp_len = datapoint_boxes.len();
        let datapoints: Vec<i64> = datapoint_boxes
            .iter()
            .map(|_| {
                unwrap_long(&datapoint_boxes[0].additional_registers.get_ordered_values()[2])
                    .unwrap_or(0)
            })
            .collect();
        // Check deviation by subtracting largest value by 2nd largest
        let front_deviation = datapoints[0] - datapoints[1];
        // Check deviation by subtracting 2nd smallest value by smallest
        let back_deviation = datapoints[dp_len - 2] - datapoints[dp_len - 1];

        // Remove largest datapoint if front deviation is greater
        if front_deviation >= back_deviation {
            processed_boxes.drain(0..1);
        }
        // Remove smallest datapoint if back deviation is greater
        else {
            processed_boxes.pop();
        }
        Ok(processed_boxes)
    }
}

// Function which produces the finalized datapoint based on a list of `ErgoBox`es.
/// If list of Datapoint boxes is outside of the deviation range then
/// attempts to filter boxes until a list which is within deviation range
/// is found.
/// Returns the averaged datapoint and the filtered list of successful boxes.
pub fn finalize_datapoint(
    boxes: &Vec<ErgoBox>,
    deviation_range: i64,
    consensus_num: i64,
) -> Result<(u64, Vec<ErgoBox>)> {
    let mut successful_boxes = boxes.clone();
    while !deviation_check(deviation_range, &successful_boxes)? {
        // Removing largest deviation outlier
        successful_boxes = remove_largest_local_deviation_datapoint(&successful_boxes)?;

        if (successful_boxes.len() as i64) < consensus_num {
            return Err(CollectionError::FailedToReachConsensus().into());
        }
    }

    // Return average + successful Datapoint boxes
    Ok((average_datapoints(&successful_boxes)?, successful_boxes))
}
