/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::encoding::{deserialize_integer, serialize_integer, serialize_string};
use crate::node_interface::{
    address_to_bytes, current_block_height, get_serialized_highest_value_unspent_box,
    send_transaction, serialize_boxes,
};
use crate::oracle_config::PoolParameters;
use crate::oracle_state::OraclePool;
use crate::templates::BASIC_TRANSACTION_SEND_REQUEST;
use json;
use sigma_tree::chain::ErgoBox;

/// The default fee used for actions
pub static FEE: u64 = 1000000;

impl OraclePool {
    /// Generates and submits the "Commit Datapoint" action tx
    pub fn action_commit_datapoint(&self, datapoint: u64) -> Option<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).ok()?;

        // Defining the registers of the output box
        let live_epoch_id = self.get_live_epoch_state()?.epoch_id;
        let registers = object! {
            "R4": address_to_bytes(&self.local_oracle_address),
            "R5": serialize_string(&live_epoch_id),
            "R6": serialize_integer(datapoint as i64)
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
        req["dataInputsRaw"] = vec![self.live_epoch_stage.get_serialized_box()].into();
        req["fee"] = FEE.into();

        send_transaction(&req)
    }

    /// Generates and submits the "Collect Funds" action tx
    pub fn action_collect_funds(&self) -> Option<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).ok()?;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": serialize_integer(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_integer(epoch_prep_state.next_epoch_ends as i64),
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
        // Only append up to 47 boxes. This is to prevent exceeding execution limit for txs.
        if initial_deposit_boxes.len() > 47 {
            unserialized_input_boxes.append(&mut initial_deposit_boxes[..47].to_vec());
        } else {
            unserialized_input_boxes.append(&mut initial_deposit_boxes);
        }
        let serialized_input_boxes = serialize_boxes(&unserialized_input_boxes);

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

        send_transaction(&req)
    }

    /// Generates and submits the "Start Next Epoch" action tx
    pub fn action_start_next_epoch(&self) -> Option<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).ok()?;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": serialize_integer(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_integer(epoch_prep_state.next_epoch_ends as i64),
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

        send_transaction(&req)
    }

    /// Generates and submits the "Create New Epoch" action tx
    pub fn action_create_new_epoch(&self) -> Option<String> {
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).ok()?;
        let parameters = PoolParameters::new();

        // Define the new epoch finish height based off of current height
        let new_finish_height = current_block_height()?
            + parameters.epoch_preparation_length
            + parameters.live_epoch_length
            + parameters.buffer_length;

        // Defining the registers of the output box
        let epoch_prep_state = self.get_preparation_state()?;
        let registers = object! {
            "R4": serialize_integer(epoch_prep_state.latest_pool_datapoint as i64),
            "R5": serialize_integer(new_finish_height as i64),
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

        send_transaction(&req)
    }

    /// Generates and submits the "Collect Datapoints" action tx
    pub fn action_collect_datapoints(&self) -> Option<String> {
        let parameters = PoolParameters::new();
        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).ok()?;

        // Defining the tokens to be spent
        let token_json = object! {
            "tokenId": self.oracle_pool_participant_token.to_string(),
            "amount": 1
        };

        // Define the finish height of the following epoch
        let new_finish_height = self.get_live_epoch_state()?.epoch_ends
            + parameters.epoch_preparation_length
            + parameters.live_epoch_length;

        // Acquire the finalized oracle pool datapoint and the list of successful datapoint boxes which were within margin of error
        let (finalized_datapoint, successful_boxes) =
            finalize_datapoint(&self.datapoint_stage.get_boxes()?)?;

        println!(
            "Finalized Datapoint: {}\nSuccessful Boxes {:?}",
            finalized_datapoint,
            successful_boxes.len()
        );

        //
        //
        //
        //

        let registers = object! {
            // "R4": ,
            // "R5": ,
            // "R6": ,
        };

        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        // Filling out the json tx request template
        req["requests"][0]["address"] =
            self.epoch_preparation_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = vec![
            self.live_epoch_stage.get_serialized_box()?,
            get_serialized_highest_value_unspent_box()?,
        ]
        .into();
        req["dataInputsRaw"] = serialize_boxes(&successful_boxes)?.into();
        req["fee"] = 3000000.into();

        None
    }
}

/// Function for averaging datapoints from a list of Datapoint boxes.
/// Returns `None` if boxes provided do not have a valid integer datapoint in R6
pub fn average_datapoints(boxes: &Vec<ErgoBox>) -> Option<u64> {
    let datapoints_sum = boxes.iter().fold(Some(0), |acc, b| {
        Some(acc? + deserialize_integer(&b.additional_registers.get_ordered_values()[2])?)
    })?;
    if boxes.len() == 0 {
        return None;
    }
    let average = datapoints_sum / boxes.len() as i64;
    Some(average as u64)
}

/// Filters out all boxes with datapoints that are greater than the margin of error
/// Returns `None` if boxes provided do not have a valid integer datapoint in R6
pub fn margin_of_error_filter(
    averaged_datapoint: u64,
    boxes: &Vec<ErgoBox>,
) -> Option<Vec<ErgoBox>> {
    // Get parameters for margin of error
    let parameters = PoolParameters::new();

    // Specifying min/max acceptable value
    let delta = (averaged_datapoint as f64 * parameters.margin_of_error) as u64;
    let min = averaged_datapoint - delta;
    let max = averaged_datapoint + delta;

    // Find the successful boxes which are within the margin of error
    let mut successful_boxes = vec![];
    for b in boxes.clone() {
        let datapoint =
            deserialize_integer(&b.additional_registers.get_ordered_values()[2])? as u64;
        if datapoint > min && datapoint < max {
            successful_boxes.push(b);
        }
    }
    Some(successful_boxes)
}

/// Function which produced the finalized datapoint based on a list of `ErgoBox`es.
/// Repeatedly acquires the average and filters out any boxes outside the margin of error.
/// Returns `None` if boxes provided do not have a valid integer datapoint in R6
pub fn finalize_datapoint(boxes: &Vec<ErgoBox>) -> Option<(u64, Vec<ErgoBox>)> {
    let mut successful_boxes = boxes.clone();
    loop {
        let av = average_datapoints(&successful_boxes)?;
        let filtered_boxes = margin_of_error_filter(av, &successful_boxes)?;
        if successful_boxes == filtered_boxes {
            return Some((av, filtered_boxes));
        }
        successful_boxes = filtered_boxes;
    }
    None
}
