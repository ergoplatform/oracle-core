/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;

use derive_more::From;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::node_interface::node_api::NodeApi;
use crate::node_interface::node_api::NodeApiError;

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
    NodeError(NodeApiError),
}

pub fn execute_action(action: PoolAction, node_api: &NodeApi) -> Result<(), ActionExecError> {
    let exec_res = match action {
        PoolAction::Refresh(action) => execute_refresh_action(action, node_api),
        PoolAction::PublishDatapoint(action) => execute_publish_datapoint_action(action, node_api),
    };
    match exec_res {
        Ok(_) => Ok(()),
        Err(ActionExecError::NodeError(NodeApiError::NodeInterfaceError(
            NodeError::BadRequest(msg),
        ))) if msg.as_str() == "Double spending attempt"
            || msg.contains("it is invalidated earlier or the pool is full")
            || msg.contains("it is already in the mempool") =>
        {
            log::debug!("Node rejected tx with error: {msg}");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn execute_refresh_action(
    action: RefreshAction,
    node_api: &NodeApi,
) -> Result<(), ActionExecError> {
    let tx_id = node_api.sign_and_submit_transaction(&action.tx)?;
    log::info!("Refresh tx published successfully, tx id: {}", tx_id);
    Ok(())
}

fn execute_publish_datapoint_action(
    action: PublishDataPointAction,
    node_api: &NodeApi,
) -> Result<(), ActionExecError> {
    let tx_id = node_api.sign_and_submit_transaction(&action.tx)?;
    log::info!("Datapoint published successfully, tx id: {}", tx_id);
    Ok(())
}
