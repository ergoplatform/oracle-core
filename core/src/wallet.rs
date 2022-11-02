use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::node_interface;

#[derive(Debug, Error, From)]
pub enum WalletDataError {
    #[error("node error: {0}")]
    NodeError(NodeError),
}

pub trait WalletDataSource {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, WalletDataError>;
}

pub struct WalletData {}

impl WalletData {
    pub fn new() -> Self {
        WalletData {}
    }
}

impl WalletDataSource for WalletData {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, WalletDataError> {
        node_interface::get_unspent_wallet_boxes().map_err(Into::into)
    }
}
