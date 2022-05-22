use ergo_lib::{
    chain::transaction::{unsigned::UnsignedTransaction, Transaction, TxIoVec},
    ergotree_ir::chain::ergo_box::ErgoBox,
};
use ergo_node_interface::node_interface::NodeError;

use crate::node_interface;

pub trait WalletDataSource {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, NodeError>;
}

pub trait WalletSign {
    fn sign_transaction_with_inputs(
        &self,
        unsigned_tx: &UnsignedTransaction,
        inputs: TxIoVec<ErgoBox>,
        data_boxes: Option<TxIoVec<ErgoBox>>,
    ) -> Result<Transaction, NodeError>;
}

pub struct WalletData {}

impl WalletData {
    pub fn new() -> Self {
        WalletData {}
    }
}

impl WalletDataSource for WalletData {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, NodeError> {
        node_interface::get_unspent_wallet_boxes()
    }
}

//impl WalletSign for WalletData {
//    fn sign_transaction_with_inputs(
//        &mut self,
//        unsigned_tx: &UnsignedTransaction,
//        _inputs: TxIoVec<ErgoBox>,
//        _data_inputs: Option<TxIoVec<ErgoBox>>,
//    ) -> Result<Transaction, NodeError> {
//        node_interface::sign_transaction(unsigned_tx)
//    }
//}
