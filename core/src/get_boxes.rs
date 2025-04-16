use crate::node_interface::node_api::{NodeApi, NodeApiError};
use crate::oracle_config::ORACLE_CONFIG;

use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

mod generic_token_fetch;
mod registry;

use crate::spec_token::TokenIdKind;
pub use generic_token_fetch::*;
pub use registry::*;

#[derive(Debug, Error)]
pub enum GetBoxesError {
    #[error("node error: {0}")]
    NodeError(#[from] NodeError),
    #[error("node api error: {0}")]
    NodeApiError(#[from] NodeApiError),
    #[error("no boxes found")]
    NoBoxesFound,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub trait GetBoxes: TokenIdKind {
    fn get_boxes(&self) -> Result<Vec<ErgoBox>, GetBoxesError> {
        let node_api = NodeApi::new(&ORACLE_CONFIG.node_url);
        let boxes = node_api.get_all_unspent_boxes_by_token_id(&self.token_id())?;
        Ok(boxes)
    }

    fn get_box(&self) -> Result<Option<ErgoBox>, GetBoxesError> {
        Ok(self.get_boxes()?.first().cloned())
    }
}
