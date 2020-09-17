/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::encoding::{
    deserialize_hex_encoded_string, deserialize_long, serialize_hex_encoded_string, serialize_int,
    serialize_long, string_to_blake2b_hash,
};
use crate::node_interface::{
    address_to_raw_for_register, address_to_tree, current_block_height,
    get_serialized_highest_value_unspent_box, raw_from_register_to_address, send_transaction,
    serialize_boxes,
};
use crate::oracle_config::PoolParameters;
use crate::oracle_state::{LiveEpochState, OraclePool};
use crate::templates::BASIC_TRANSACTION_SEND_REQUEST;
use crate::Result;
use anyhow::anyhow;
use json;
use sigma_tree::chain::ErgoBox;

impl OraclePool {
    /// Generates and submits the "Commit Datapoint" action tx
    pub fn action_commit_datapoint(&self, datapoint: u64) -> Result<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Defining the registers of the output box
        let live_epoch_id = self.get_live_epoch_state()?.epoch_id;
        let registers = object! {
            "R4": address_to_raw_for_register(&self.local_oracle_address)?,
            "R5": serialize_hex_encoded_string(&live_epoch_id)?,
            "R6": serialize_long(datapoint as i64),
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_participant_token.to_string(),
            "amount": 1
        };

        // Filling out the json tx request template
        req["requests"][0]["address"] = self.datapoint_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = vec![
            self.local_oracle_datapoint_scan.get_serialized_box()?,
            get_serialized_highest_value_unspent_box()?,
        ]
        .into();
        req["dataInputsRaw"] = vec![self.live_epoch_stage.get_serialized_box()?].into();
        req["fee"] = parameters.base_fee.into();
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
            "R4": serialize_long(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_int(epoch_prep_state.next_epoch_ends as i32),
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
        // Serialize boxes and add extra box for paying fee
        let mut serialized_input_boxes = serialize_boxes(&unserialized_input_boxes)?;
        serialized_input_boxes.push(get_serialized_highest_value_unspent_box()?);

        // Define the fee for the current action
        let action_fee = 500000 * unserialized_input_boxes.len() as u64;

        // Sum up the new total minus tx fee
        let total_input_ergs = unserialized_input_boxes
            .iter()
            .fold(0, |acc, b| acc + b.value.value());

        // Filling out the json tx request template
        req["requests"][0]["value"] = total_input_ergs.into();
        req["requests"][0]["address"] =
            self.epoch_preparation_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
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
            "R4": serialize_long(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_int(epoch_prep_state.next_epoch_ends as i32),
            "R6": serialize_hex_encoded_string(&string_to_blake2b_hash(address_to_tree(&self.epoch_preparation_stage.contract_address)?)?)?,
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_nft.to_string(),
            "amount": 1
        };

        // Filling out the json tx request template
        req["requests"][0]["value"] = epoch_prep_state.funds.into();
        req["requests"][0]["address"] = self.live_epoch_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = vec![
            self.epoch_preparation_stage.get_serialized_box()?,
            get_serialized_highest_value_unspent_box()?,
        ]
        .into();
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
            "R4": serialize_long(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_int(new_finish_height as i32),
            "R6": serialize_hex_encoded_string(&string_to_blake2b_hash(address_to_tree(&self.epoch_preparation_stage.contract_address)?)?)?,
        };
        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_nft.to_string(),
            "amount": 1
        };

        // Filling out the json tx request template
        req["requests"][0]["value"] = epoch_prep_state.funds.into();
        req["requests"][0]["address"] = self.live_epoch_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = vec![
            self.epoch_preparation_stage.get_serialized_box()?,
            get_serialized_highest_value_unspent_box()?,
        ]
        .into();
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
        // Find the index of the local oracle's Datapoint box in the sorted list
        let local_datapoint_box_index = find_box_index_in_list(
            self.local_oracle_datapoint_scan.get_box()?,
            sorted_datapoint_boxes,
        );

        // Acquire the finalized oracle pool datapoint and the list of successful datapoint boxes which were within outlier range
        let (finalized_datapoint, successful_boxes) = finalize_datapoint(
            &sorted_datapoint_boxes,
            live_epoch_state.latest_pool_datapoint,
        )?;

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
            "R4": serialize_long(finalized_datapoint as i64),
            "R5": serialize_int(new_finish_height as i32),
        };

        req["requests"][0]["value"] = new_box_value.into();
        req["requests"][0]["address"] =
            self.epoch_preparation_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
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

        // Filling out request for collector payout
        req["requests"]
            .push(object! {
                "address": self.local_oracle_address.clone(),
                "value": parameters.oracle_payout_price,
            })
            .ok();

        // Filling out the rest of the json request
        req["inputsRaw"] = vec![
            self.live_epoch_stage.get_serialized_box()?,
            get_serialized_highest_value_unspent_box()?,
        ]
        .into();
        req["dataInputsRaw"] = serialize_boxes(&successful_boxes)?.into();
        req["fee"] = tx_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }
}

/// Given an `ErgoBox`, find its index in the input `Vec<ErgoBox>`
fn find_box_index_in_list(search_box: ErgoBox, sorted_datapoint_boxes: Vec<ErgoBox>) {
    todo!()
}

/// Filters out Datapoint boxes that are not from the current epoch
pub fn current_epoch_boxes_filter(
    datapoint_boxes: &Vec<ErgoBox>,
    live_epoch_state: &LiveEpochState,
) -> Vec<ErgoBox> {
    let mut filtered_boxes = vec![];
    for b in datapoint_boxes {
        if let Ok(s) =
            deserialize_hex_encoded_string(&b.additional_registers.get_ordered_values()[1])
        {
            if s == live_epoch_state.epoch_id {
                filtered_boxes.push(b.clone());
            }
        }
    }
    filtered_boxes
}

/// Sort Datapoint boxes in decreasing order based on Datapoint value.
pub fn sort_datapoint_boxes(all_datapoint_boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
    let mut datapoint_boxes = all_datapoint_boxes.clone();
    datapoint_boxes.sort_by_key(|b| {
        deserialize_long(&b.additional_registers.get_ordered_values()[2]).unwrap_or(0)
    });
    datapoint_boxes
}

/// Function for averaging datapoints from a list of Datapoint boxes.
pub fn average_datapoints(boxes: &Vec<ErgoBox>) -> Result<u64> {
    let datapoints_sum = boxes.iter().fold(Ok(0), |acc: Result<i64>, b| {
        Ok(acc? + deserialize_long(&b.additional_registers.get_ordered_values()[2])?)
    })?;
    if boxes.len() == 0 {
        return Err(anyhow!("No datapoints posted in current epoch."));
    }
    let average = datapoints_sum / boxes.len() as i64;
    Ok(average as u64)
}

/// Filters out all boxes with datapoints that are outside of the outlier range compared to the latest Oracle Pool finalized datapoint
pub fn outlier_range_filter(
    boxes: &Vec<ErgoBox>,
    latest_finalized_datapoint: u64,
) -> Result<Vec<ErgoBox>> {
    // Get parameters for outlier range
    let parameters = PoolParameters::new();

    // Specifying min/max acceptable value
    let delta = (latest_finalized_datapoint / 100) * parameters.outlier_range;
    let min = latest_finalized_datapoint - delta;
    let max = latest_finalized_datapoint + delta;

    // Find the successful boxes which are within the outlier range
    let mut successful_boxes = vec![];
    for b in boxes.clone() {
        let datapoint = deserialize_long(&b.additional_registers.get_ordered_values()[2])? as u64;
        if datapoint >= min && datapoint <= max {
            successful_boxes.push(b);
        }
    }
    Ok(successful_boxes)
}

/// Removes boxes which do not have a valid datapoint Long in R6.
pub fn valid_boxes_filter(boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
    let mut valid_boxes = vec![];
    for b in boxes {
        if let Ok(_) = deserialize_long(&b.additional_registers.get_ordered_values()[2]) {
            valid_boxes.push(b.clone());
        }
    }
    valid_boxes
}

/// Function which produces the finalized datapoint based on a list of `ErgoBox`es.
/// Filters out any invalid boxes or boxes outside the outlier range.
/// Returns the averaged datapoint and the filtered list of successful boxes.
pub fn finalize_datapoint(
    boxes: &Vec<ErgoBox>,
    latest_finalized_datapoint: u64,
) -> Result<(u64, Vec<ErgoBox>)> {
    // Filter out Datapoint boxes without a valid integer in R6
    let valid_boxes = valid_boxes_filter(boxes);
    // Filter out Datapoint boxes outside of the outlier range
    let successful_boxes = outlier_range_filter(&valid_boxes, latest_finalized_datapoint)?;
    // Return average
    Ok((average_datapoints(&successful_boxes)?, successful_boxes))
}
