use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::node_interface::node_api::NodeApiError;

#[derive(Debug, Error)]
pub enum WalletDataError {
    #[error("node error: {0}")]
    NodeError(#[from] NodeError),
    #[error("no change address found")]
    NoChangeAddressSetInNode,
    #[error("AddressEncoder error: {0}")]
    AddressEncoder(#[from] AddressEncoderError),
    #[error("node api error: {0}")]
    NodeApiError(#[from] NodeApiError),
}

// TODO: remove and pass unspent boxes and change address directly?
pub trait WalletDataSource {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, WalletDataError>;
    fn get_change_address(&self) -> Result<NetworkAddress, WalletDataError>;
}
