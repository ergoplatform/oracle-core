/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::encoding::{
    deserialize_ergo_tree, deserialize_long, deserialize_string, serialize_hex_encoded_string,
    serialize_int, serialize_long,
};
use crate::node_interface::{
    address_to_bytes, current_block_height, get_serialized_highest_value_unspent_box,
    send_transaction, serialize_boxes,
};
use crate::oracle_config::PoolParameters;
use crate::oracle_state::{LiveEpochState, OraclePool};
use crate::templates::BASIC_TRANSACTION_SEND_REQUEST;
use crate::Result;
use anyhow::anyhow;
use json;
use sigma_tree::chain::ErgoBox;

/// The default fee used for actions
pub static FEE: u64 = 1000000;

impl OraclePool {
    /// Generates and submits the "Commit Datapoint" action tx
    pub fn action_commit_datapoint(&self, datapoint: u64) -> Result<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Defining the registers of the output box
        let live_epoch_id = self.get_live_epoch_state()?.epoch_id;
        let registers = object! {
            "R4": address_to_bytes(&self.local_oracle_address)?,
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
        req["fee"] = FEE.into();

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
            "R5": serialize_long(epoch_prep_state.next_epoch_ends as i64),
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
        let serialized_input_boxes = serialize_boxes(&unserialized_input_boxes)?;

        // Define the fee for the current action
        let action_fee = 8000000;

        // Sum up the new total minus tx fee
        let total_input_ergs = unserialized_input_boxes
            .iter()
            .fold(0, |acc, b| acc + b.value.value());
        let nano_ergs_sum = total_input_ergs - action_fee;

        // Filling out the json tx request template
        req["requests"][0]["value"] = nano_ergs_sum.into();
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
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": serialize_long(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_int(epoch_prep_state.next_epoch_ends as i32),
            "R6": address_to_bytes(&self.epoch_preparation_stage.contract_address)?,
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
        req["fee"] = FEE.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }

    /// Generates and submits the "Create New Epoch" action tx
    pub fn action_create_new_epoch(&self) -> Result<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;
        let parameters = PoolParameters::new();

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
            "R6": address_to_bytes(&self.epoch_preparation_stage.contract_address)?,
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
        req["fee"] = (FEE * 2).into();

        let result = send_transaction(&req)?;
        Ok(result)
    }

    /// Generates and submits the "Collect Datapoints" action tx
    pub fn action_collect_datapoints(&self) -> Result<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

        let live_epoch_state = self.get_live_epoch_state()?;

        // Filter out Datapoint boxes not from the latest epoch.
        let current_epoch_datapoint_boxes =
            current_epoch_boxes_filter(&self.datapoint_stage.get_boxes()?, &live_epoch_state);

        // Acquire the finalized oracle pool datapoint and the list of successful datapoint boxes which were within margin of error
        let (finalized_datapoint, successful_boxes) =
            finalize_datapoint(&current_epoch_datapoint_boxes)?;

        // Tx fee for the transaction
        let tx_fee = 5000000;
        // Define the new value of the oracle pool box after payouts/tx fee
        let new_box_value = live_epoch_state.funds
            - (parameters.oracle_payout_price * (successful_boxes.len() as u64 + 1))
            - tx_fee;
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
            let oracle_address =
                deserialize_ergo_tree(&b.additional_registers.get_ordered_values()[0])?;
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
        req["inputsRaw"] = vec![self.live_epoch_stage.get_serialized_box()?].into();
        req["dataInputsRaw"] = serialize_boxes(&successful_boxes)?.into();
        req["fee"] = tx_fee.into();

        let result = send_transaction(&req)?;
        Ok(result)
    }
}

/// Filters out Datapoint boxes that are not from the current epoch
pub fn current_epoch_boxes_filter(
    datapoint_boxes: &Vec<ErgoBox>,
    live_epoch_state: &LiveEpochState,
) -> Vec<ErgoBox> {
    let mut filtered_boxes = vec![];
    for b in datapoint_boxes {
        if let Ok(s) = deserialize_string(&b.additional_registers.get_ordered_values()[1]) {
            if s == live_epoch_state.epoch_id {
                filtered_boxes.push(b.clone());
            }
        }
    }
    filtered_boxes
}

/// Function for averaging datapoints from a list of Datapoint boxes.
/// Returns `None` if boxes provided do not have a valid integer datapoint in R6
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

/// Filters out all boxes with datapoints that are greater than the margin of error
/// Returns `None` if boxes provided do not have a valid integer datapoint in R6
pub fn margin_of_error_filter(
    averaged_datapoint: u64,
    boxes: &Vec<ErgoBox>,
) -> Result<Vec<ErgoBox>> {
    // Get parameters for margin of error
    let parameters = PoolParameters::new();

    // Specifying min/max acceptable value
    let delta = (averaged_datapoint as f64 * parameters.margin_of_error) as u64;
    let min = averaged_datapoint - delta;
    let max = averaged_datapoint + delta;

    // Find the successful boxes which are within the margin of error
    let mut successful_boxes = vec![];
    for b in boxes.clone() {
        let datapoint = deserialize_long(&b.additional_registers.get_ordered_values()[2])? as u64;
        if datapoint > min && datapoint < max {
            successful_boxes.push(b);
        }
    }
    Ok(successful_boxes)
}

/// Removes boxes which do not have a valid address in R4 and datapoint integer in R6.
/// This is to prevent returning `None` while finalizing the datapoint and thereby
/// actually build a tx that validates.
pub fn valid_boxes_filter(boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
    let mut valid_boxes = vec![];
    for b in boxes {
        if let Ok(_) = deserialize_ergo_tree(&b.additional_registers.get_ordered_values()[0]) {
            if let Ok(_) = deserialize_long(&b.additional_registers.get_ordered_values()[2]) {
                valid_boxes.push(b.clone());
            }
        }
    }
    valid_boxes
}

/// Function which produced the finalized datapoint based on a list of `ErgoBox`es.
/// Repeatedly acquires the average and filters out any boxes outside the margin of error.
/// Returns `None` if boxes provided do not have a valid integer datapoint in R6
pub fn finalize_datapoint(boxes: &Vec<ErgoBox>) -> Result<(u64, Vec<ErgoBox>)> {
    // Filter out Datapoint boxes without a valid integer in R6
    let mut successful_boxes = valid_boxes_filter(boxes);
    Ok((average_datapoints(&successful_boxes)?, successful_boxes))

    // Logic for outlier checking to be integrated later on.
    // For now just take straight average of datapoints, no outlier checking so commented out.
    //
    // loop {
    //     let av = average_datapoints(&successful_boxes)?;
    //     let filtered_boxes = margin_of_error_filter(av, &successful_boxes)?;
    //     if successful_boxes == filtered_boxes {
    //         return Ok((av, filtered_boxes));
    //     }
    //     successful_boxes = filtered_boxes;
    // }
    // None
}
