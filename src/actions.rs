/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::oracle_state::{OraclePool};
use crate::node_interface::{send_transaction};

impl OraclePool {
    /// Generates and submits the 'Commit Datapoint" action tx
    // To do: Datapoint type based off of type specified in oracle-config.yaml
    pub fn action_commit_datapoint(&self, datapoint: u64) -> Option<String> {


        let tx_request = object!{
            address: "",
            };

        send_transaction(&tx_request)
    }
}