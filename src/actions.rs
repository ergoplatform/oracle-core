use crate::encoding::{serialize_integer, serialize_string};
use crate::node_interface::{
    address_to_bytes, get_serialized_highest_value_unspent_box, send_transaction, serialize_box,
    serialize_boxes,
};
/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::oracle_state::OraclePool;
use crate::templates::BASIC_TRANSACTION_SEND_REQUEST;
use json;
use sigma_tree::chain::ErgoBox;

/// The default fee used for actions
pub static FEE: u64 = 1000000;

impl OraclePool {
    /// Generates and submits the 'Commit Datapoint" action tx
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

        // Finding a box with Ergs in the wallet to fund the tx and used in inputsRaw
        let ergs_box_serialized = get_serialized_highest_value_unspent_box()?;

        // Filling out the json tx request template
        req["requests"][0]["address"] = self.datapoint_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = vec![
            self.datapoint_stage.get_serialized_box()?,
            ergs_box_serialized,
        ]
        .into();
        req["dataInputsRaw"] = vec![self.live_epoch_stage.get_serialized_box()].into();
        req["fee"] = FEE.into();

        send_transaction(&req)
    }

    /// Generates and submits the 'Collect Funds" action tx
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
        // Only append up to 10 boxes. This is to prevent exceeding execution limit for txs.
        if initial_deposit_boxes.len() > 20 {
            unserialized_input_boxes.append(&mut initial_deposit_boxes[..18].to_vec());
        } else {
            unserialized_input_boxes.append(&mut initial_deposit_boxes);
        }
        let serialized_input_boxes = serialize_boxes(&unserialized_input_boxes);

        // Sum up the new total minus tx fee
        let total_input_ergs = unserialized_input_boxes
            .iter()
            .fold(0, |acc, b| acc + b.value.value());
        let nano_ergs_sum = total_input_ergs - FEE;

        // Filling out the json tx request template
        req["requests"][0]["value"] = nano_ergs_sum.into();
        req["requests"][0]["address"] =
            self.epoch_preparation_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = serialized_input_boxes.into();
        req["fee"] = (FEE * 9).into();

        send_transaction(&req)
    }
}
