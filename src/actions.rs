/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::oracle_state::{OraclePool};
use crate::node_interface::{send_transaction};
use crate::templates::{BASIC_TRANSACTION_SEND_REQUEST};
use json;

impl OraclePool {
    /// Generates and submits the 'Commit Datapoint" action tx
    pub fn action_commit_datapoint(&self, datapoint: u64) -> Option<String> {

        let mut req = json::parse(BASIC_TRANSACTION_SEND_REQUEST).ok()?;


        req["requests"]["address"] = self.datapoint_stage.contract_address.clone().into();

        println!("{:?}", req.dump());

        // send_transaction(&req)
        None
    }
}