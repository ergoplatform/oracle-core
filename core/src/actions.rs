/// This file holds all the actions which can be performed
/// by an oracle part of the oracle pool. These actions
/// are implemented on the `OraclePool` struct.
use crate::node_interface::sign_and_submit_transaction;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;

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
        PoolAction::Refresh(action) => {
            log::info!("Executing refresh action: {:?}", action);
            execute_refresh_action(action)
        }
        PoolAction::PublishDatapoint(action) => {
            log::info!("Executing publish datapoint action: {:?}", action);
            execute_publish_datapoint_action(action)
        }
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
