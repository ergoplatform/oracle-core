use crate::contracts::pool::PoolContractError;
use crate::contracts::refresh::RefreshContractError;
use crate::node_interface::node_api::{NodeApi, NodeApiError};
use crate::oracle_config::ORACLE_CONFIG;

use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use ergo_node_interface::ScanId;
use thiserror::Error;

mod generic_token_scan;
mod registry;

pub use generic_token_scan::*;
pub use registry::*;

/// Integer which is provided by the Ergo node to reference a given scan.
pub type ScanID = String;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("node error: {0}")]
    NodeError(#[from] NodeError),
    #[error("node api error: {0}")]
    NodeApiError(#[from] NodeApiError),
    #[error("no boxes found")]
    NoBoxesFound,
    #[error("failed to register scan")]
    FailedToRegister,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("refresh contract error: {0}")]
    RefreshContract(#[from] RefreshContractError),
    #[error("pool contract error: {0}")]
    PoolContract(#[from] PoolContractError),
}

pub trait NodeScanId {
    fn scan_id(&self) -> ScanId;
}

pub trait ScanGetBoxes: NodeScanId {
    fn get_boxes(&self) -> Result<Vec<ErgoBox>, ScanError> {
        let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
        let boxes = node_api.node.scan_boxes(self.scan_id())?;
        Ok(boxes)
    }

    fn get_box(&self) -> Result<Option<ErgoBox>, ScanError> {
        Ok(self.get_boxes()?.first().cloned())
    }
}
