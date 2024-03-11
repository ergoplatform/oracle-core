use crate::node_interface::node_api::NodeApi;
use ergo_lib::{
    chain::transaction::{unsigned::UnsignedTransaction, Transaction, TxId, TxIoVec},
    ergotree_ir::chain::ergo_box::ErgoBox,
};
use ergo_node_interface::node_interface::{NodeError, NodeInterface};
use log::debug;
use log::error;

pub mod node_api;

pub type Result<T> = std::result::Result<T, NodeError>;

pub trait SubmitTransaction {
    fn submit_transaction(&self, tx: &Transaction) -> Result<TxId>;
}

pub trait SignTransactionWithInputs {
    fn sign_transaction_with_inputs(
        &self,
        unsigned_tx: &UnsignedTransaction,
        inputs: TxIoVec<ErgoBox>,
        data_boxes: Option<TxIoVec<ErgoBox>>,
    ) -> Result<Transaction>;
}

pub trait SignTransaction {
    fn sign_transaction(&self, unsigned_tx: &UnsignedTransaction) -> Result<Transaction>;
}

// Note that we need the following trait implementations for `NodeInterface` because we can't rely
// on any of the functions in the `crate::node_interface` module since they all implicitly rely on
// the existence of an oracle-pool `yaml` config file.

impl SignTransaction for NodeInterface {
    fn sign_transaction(&self, unsigned_tx: &UnsignedTransaction) -> Result<Transaction> {
        self.sign_transaction(unsigned_tx, None, None)
    }
}

impl SubmitTransaction for NodeInterface {
    fn submit_transaction(&self, tx: &Transaction) -> crate::node_interface::Result<TxId> {
        log::trace!(
            "Submitting signed transaction: {}",
            serde_json::to_string_pretty(&tx).unwrap()
        );
        self.submit_transaction(tx)
    }
}

impl SignTransactionWithInputs for NodeInterface {
    fn sign_transaction_with_inputs(
        &self,
        unsigned_tx: &ergo_lib::chain::transaction::unsigned::UnsignedTransaction,
        inputs: ergo_lib::chain::transaction::TxIoVec<ErgoBox>,
        data_boxes: Option<ergo_lib::chain::transaction::TxIoVec<ErgoBox>>,
    ) -> Result<Transaction> {
        self.sign_transaction(
            unsigned_tx,
            Some(inputs.as_vec().clone()),
            data_boxes.map(|bs| bs.as_vec().clone()),
        )
    }
}

pub fn try_ensure_wallet_unlocked(node: &NodeApi) {
    let unlocked = node.node.wallet_status().unwrap().unlocked;

    if !unlocked {
        if let Some(wallet_pass) = &node.wallet_pass {
            if let Err(e) = node.wallet_unlock(wallet_pass) {
                error!("Failed to unlock wallet. Wallet must be unlocked for node operations. error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        } else {
            error!("Wallet must be unlocked for node operations");
            std::process::exit(exitcode::SOFTWARE);
        }
    } else {
        debug!("Wallet unlocked");
    }
}
