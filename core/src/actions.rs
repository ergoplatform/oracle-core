/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::node_interface::sign_and_submit_transaction;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;

use derive_more::From;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

mod collect;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, From)]
pub enum PoolAction {
    Refresh(RefreshAction),
    PublishDatapoint(PublishDataPointAction),
}

#[derive(Debug)]
pub struct RefreshAction {
    pub tx: UnsignedTransaction,
}

#[derive(Debug)]
pub struct PublishDataPointAction {
    pub tx: UnsignedTransaction,
}

#[derive(Error, Debug, From)]
pub enum ActionExecError {
    #[error("node error: {0}")]
    NodeError(NodeError),
}

pub fn execute_action(action: PoolAction) -> Result<(), ActionExecError> {
    let exec_res = match action {
        PoolAction::Refresh(action) => execute_refresh_action(action),
        PoolAction::PublishDatapoint(action) => execute_publish_datapoint_action(action),
    };
    match exec_res {
        Ok(_) => Ok(()),
        Err(ActionExecError::NodeError(NodeError::BadRequest(msg)))
            if msg.as_str() == "Double spending attempt"
                || msg.contains("it is invalidated earlier or the pool is full") =>
        {
            log::info!(
                "Node rejected tx, probably due to our previous tx is still in the mempool)"
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn execute_refresh_action(action: RefreshAction) -> Result<(), ActionExecError> {
    let tx_id = sign_and_submit_transaction(&action.tx)?;
    log::info!("Refresh tx published successfully, tx id: {}", tx_id);
    Ok(())
}

fn execute_publish_datapoint_action(action: PublishDataPointAction) -> Result<(), ActionExecError> {
    let tx_id = sign_and_submit_transaction(&action.tx)?;
    log::info!("Datapoint published successfully, tx id: {}", tx_id);
    Ok(())
}
