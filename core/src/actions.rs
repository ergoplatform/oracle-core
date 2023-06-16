/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;

use derive_more::From;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::explorer_api::ergo_explorer_transaction_link;
use crate::node_interface::node_api::NodeApi;
use crate::node_interface::node_api::NodeApiError;
use crate::oracle_config::ORACLE_CONFIG;

mod action_result;

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

#[derive(Error, Debug)]
pub enum ActionExecError {
    #[error("node error: {0}")]
    NodeError(#[from] NodeApiError),
}

pub fn execute_action(action: PoolAction, node_api: &NodeApi) -> Result<(), anyhow::Error> {
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
            || msg.contains("it is already in the mempool")
            || msg.contains("Not enough boxes to spend") // node cannot find all the input boxes due to them being spent in previous tx (last main loop iteration), see https://github.com/ergoplatform/oracle-core/issues/220
            =>
        {
            log::debug!("Node rejected tx with error: {msg}");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

fn execute_refresh_action(
    action: RefreshAction,
    node_api: &NodeApi,
) -> Result<(), ActionExecError> {
    let tx_id = node_api.sign_and_submit_transaction(&action.tx)?;
    let network_prefix = &ORACLE_CONFIG.oracle_address.network();
    log::info!(
        "Refresh tx published. Check status: {}",
        ergo_explorer_transaction_link(tx_id, *network_prefix)
    );
    Ok(())
}

fn execute_publish_datapoint_action(
    action: PublishDataPointAction,
    node_api: &NodeApi,
) -> Result<(), ActionExecError> {
    let tx_id = node_api.sign_and_submit_transaction(&action.tx)?;
    let network_prefix = &ORACLE_CONFIG.oracle_address.network();
    log::info!(
        "Datapoint tx published. Check status: {}",
        ergo_explorer_transaction_link(tx_id, *network_prefix)
    );
    Ok(())
}
