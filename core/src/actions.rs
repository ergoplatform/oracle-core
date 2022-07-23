/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::node_interface::sign_and_submit_transaction;
use crate::oracle_state::OraclePool;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;

use derive_more::From;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

mod collect;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, From)]
pub enum PoolAction {
    Bootstrap(BootstrapAction),
    Refresh(RefreshAction),
    PublishDatapoint(PublishDataPointAction),
}

#[derive(Debug)]
pub struct BootstrapAction {}

#[derive(Debug)]
pub struct RefreshAction {
    pub tx: UnsignedTransaction,
}

#[derive(Debug)]
pub struct PublishDataPointAction {
    pub tx: UnsignedTransaction,
}

#[derive(Error, Debug)]
pub enum CollectionError {
    #[error("Failed collecting datapoints. The minimum consensus number could not be reached, meaning that an insufficient number of oracles posted datapoints within the deviation range.")]
    FailedToReachConsensus(),
    #[error("Failed collecting datapoints. The local oracle did not post a datapoint in the current epoch.")]
    LocalOracleFailedToPostDatapoint(),
    #[error("Failed collecting datapoints. The local oracle did not post a datapoint within the deviation range (when compared to datapoints posted by other oracles in the pool).")]
    LocalOracleFailedToPostDatapointWithinDeviation(),
}

#[derive(Error, Debug, From)]
pub enum ActionExecError {
    #[error("node error: {0}")]
    NodeError(NodeError),
}

pub fn execute_action(action: PoolAction) -> Result<(), ActionExecError> {
    match action {
        PoolAction::Bootstrap(_) => todo!(),
        PoolAction::Refresh(action) => execute_refresh_action(action),
        PoolAction::PublishDatapoint(action) => execute_publish_datapoint_action(action),
    }
}

fn execute_refresh_action(action: RefreshAction) -> Result<(), ActionExecError> {
    let _tx_id = sign_and_submit_transaction(&action.tx)?;
    Ok(())
}

fn execute_publish_datapoint_action(action: PublishDataPointAction) -> Result<(), ActionExecError> {
    let _tx_id = sign_and_submit_transaction(&action.tx)?;
    Ok(())
}

impl<'a> OraclePool<'a> {
    // /// Generates and submits the "Collect Funds" action tx
    // pub fn action_collect_funds(&self) -> Result<String, StageError> {
    //     let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).unwrap();

    //     // Defining the registers of the output box
    //     let epoch_prep_state = self.get_preparation_state()?;
    //     let registers = object! {
    //         "R4": Constant::from(epoch_prep_state.latest_pool_datapoint as i64).base16_str().unwrap(),

    //         "R5": Constant::from(epoch_prep_state.next_epoch_ends as i32).base16_str().unwrap(),
    //     };
    //     // Defining the tokens to be spent
    //     let token_json = object! {
    //         "tokenId": self.oracle_pool_nft.to_string(),
    //         "amount": 1
    //     };

    //     // Create input boxes Vec with serialized Epoch Preparation box inside
    //     let mut unserialized_input_boxes = vec![self.epoch_preparation_stage.get_box()?];
    //     // Acquire all Pool Deposit boxes
    //     let mut initial_deposit_boxes = self.pool_deposit_stage.get_boxes()?;
    //     // Only append up to 27 boxes for now. This is to prevent exceeding execution limit for txs.
    //     if initial_deposit_boxes.len() > 27 {
    //         unserialized_input_boxes.append(&mut initial_deposit_boxes[..27].to_vec());
    //     } else {
    //         unserialized_input_boxes.append(&mut initial_deposit_boxes);
    //     }

    //     // Define the fee for the current action
    //     let action_fee = 500000 * unserialized_input_boxes.len() as u64;

    //     // Serialize boxes and add extra box for paying fee
    //     let mut serialized_input_boxes = serialize_boxes(&unserialized_input_boxes)?;
    //     serialized_input_boxes.append(&mut serialized_unspent_boxes_with_min_total(action_fee)?);

    //     // Sum up the new total minus tx fee
    //     let total_input_ergs = unserialized_input_boxes
    //         .iter()
    //         .fold(0, |acc, b| acc + b.value.as_u64());

    //     // Filling out the json tx request template
    //     req["requests"][0]["value"] = total_input_ergs.into();
    //     req["requests"][0]["address"] =
    //         self.epoch_preparation_stage.contract_address.clone().into();
    //     req["requests"][0]["registers"] = registers;
    //     req["requests"][0]["assets"] = vec![token_json].into();
    //     req["inputsRaw"] = serialized_input_boxes.into();
    //     req["fee"] = action_fee.into();

    //     let result = send_transaction(&req)?;
    //     Ok(result)
    // }

    // /// Generates and submits the "Start Next Epoch" action tx
    // pub fn action_start_next_epoch(&self) -> Result<String, StageError> {
    // let parameters = PoolParameters::new();
    // let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

    // // Defining the registers of the output box
    // let epoch_prep_state = self.get_preparation_state()?;
    // let registers = object! {
    //     "R4": Constant::from(epoch_prep_state.latest_pool_datapoint as i64).base16_str().unwrap(),
    //     "R5": Constant::from(epoch_prep_state.next_epoch_ends as i32).base16_str().unwrap(),
    //     "R6": serialize_hex_encoded_string(&string_to_blake2b_hash(address_to_tree(&self.epoch_preparation_stage.contract_address)?)?)?.base16_str().unwrap(),
    // };
    // // Defining the tokens to be spent
    // let token_json = object! {
    //     "tokenId": self.oracle_pool_nft.to_string(),
    //     "amount": 1
    // };

    // let mut inputs_raw = vec![self.epoch_preparation_stage.get_serialized_box()?];
    // inputs_raw.append(&mut serialized_unspent_boxes_with_min_total(
    //     parameters.base_fee,
    // )?);

    // // Filling out the json tx request template
    // req["requests"][0]["value"] = epoch_prep_state.funds.into();
    // req["requests"][0]["address"] = self.live_epoch_stage.contract_address.clone().into();
    // req["requests"][0]["registers"] = registers;
    // req["requests"][0]["assets"] = vec![token_json].into();
    // req["inputsRaw"] = inputs_raw.into();
    // req["fee"] = parameters.base_fee.into();

    // let result = send_transaction(&req)?;
    // Ok(result)
    // }

    // /// Generates and submits the "Create New Epoch" action tx
    // pub fn action_create_new_epoch(&self) -> Result<String, StageError> {
    // let parameters = PoolParameters::new();
    // let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST)?;

    // // Define the new epoch finish height based off of current height
    // let new_finish_height = current_block_height()?
    //     + parameters.epoch_preparation_length
    //     + parameters.live_epoch_length
    //     + parameters.buffer_length;

    // // Defining the registers of the output box
    // let epoch_prep_state = self.get_preparation_state()?;
    // let registers = object! {
    //     "R4": Constant::from(epoch_prep_state.latest_pool_datapoint as i64).base16_str().unwrap(),
    //     "R5": Constant::from(new_finish_height as i32).base16_str().unwrap(),
    //     "R6": serialize_hex_encoded_string(&string_to_blake2b_hash(address_to_tree(&self.epoch_preparation_stage.contract_address)?)?)?.base16_str().unwrap(),
    // };
    // // Defining the tokens to be spent
    // let token_json = object! {
    //     "tokenId": self.oracle_pool_nft.to_string(),
    //     "amount": 1
    // };

    // let mut inputs_raw = vec![self.epoch_preparation_stage.get_serialized_box()?];
    // inputs_raw.append(&mut serialized_unspent_boxes_with_min_total(
    //     parameters.base_fee,
    // )?);

    // // Filling out the json tx request template
    // req["requests"][0]["value"] = epoch_prep_state.funds.into();
    // req["requests"][0]["address"] = self.live_epoch_stage.contract_address.clone().into();
    // req["requests"][0]["registers"] = registers;
    // req["requests"][0]["assets"] = vec![token_json].into();
    // req["inputsRaw"] = inputs_raw.into();
    // req["fee"] = parameters.base_fee.into();

    // let result = send_transaction(&req)?;
    // Ok(result)
    // }

    /*
    /// Generates and submits the "Collect Datapoints" action tx
    pub fn action_collect_datapoints(&self) -> Result<String, StateError> {
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
            "R4": Constant::from(finalized_datapoint as i64).base16_str().unwrap(),
            "R5": Constant::from(new_finish_height as i32).base16_str().unwrap(),
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
                &b.additional_registers.get_ordered_values()[0].base16_str().unwrap(),
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
            "R4": Constant::from(local_datapoint_box_index as i32).base16_str().unwrap()
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
    */
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
