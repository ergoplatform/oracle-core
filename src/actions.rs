use crate::encoding::{serialize_integer, serialize_string};
use crate::node_interface::{
    address_to_bytes, get_highest_value_unspent_box, send_transaction, serialized_box_from_id,
};
/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::oracle_state::OraclePool;
use crate::templates::BASIC_TRANSACTION_SEND_REQUEST;
use json;
use sigma_tree::chain::ErgoBox;

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
        let ergs_box_id: String = get_highest_value_unspent_box()?.box_id().into();
        let ergs_box_serialized = serialized_box_from_id(&ergs_box_id)?;

        // Getting the serialized boxes
        let datapoint_box_serialized =
            serialized_box_from_id(&self.datapoint_stage.get_box()?.box_id().into())?;
        let live_epoch_box_serialized = serialized_box_from_id(&live_epoch_id)?;

        // Filling out the json tx request template
        req["requests"][0]["address"] = self.datapoint_stage.contract_address.clone().into();
        req["requests"][0]["registers"] = registers.into();
        req["requests"][0]["assets"] = vec![token_json].into();
        req["inputsRaw"] = vec![datapoint_box_serialized, ergs_box_serialized].into();
        req["dataInputsRaw"] = vec![live_epoch_box_serialized].into();

        println!("{:?}", json::stringify(req.clone()));

        send_transaction(&req)
        // None
    }
}
