use crate::{
    oracle_config::{get_node_api_key, get_node_ip, get_node_port},
    wallet::WalletDataSource,
};
use ergo_lib::{
    chain::transaction::{unsigned::UnsignedTransaction, Transaction, TxIoVec},
    ergotree_ir::chain::ergo_box::ErgoBox,
};
use ergo_node_interface::{
    node_interface::{NodeError, NodeInterface, WalletStatus},
    BlockHeight,
};
use log::debug;
use log::error;

pub type Result<T> = std::result::Result<T, NodeError>;
pub type ScanID = String;
pub type TxId = String;
pub type P2PKAddressString = String;
pub type P2SAddressString = String;

pub trait SubmitTransaction {
    fn submit_transaction(&self, tx: &Transaction) -> Result<String>;
}

pub trait SignTransaction {
    fn sign_transaction_with_inputs(
        &self,
        unsigned_tx: &UnsignedTransaction,
        inputs: TxIoVec<ErgoBox>,
        data_boxes: Option<TxIoVec<ErgoBox>>,
    ) -> Result<Transaction>;
}

// Note that we need the following trait implementations for `NodeInterface` because we can't rely
// on any of the functions in the `crate::node_interface` module since they all implicitly rely on
// the existence of an oracle-pool `yaml` config file.

impl SubmitTransaction for NodeInterface {
    fn submit_transaction(&self, tx: &Transaction) -> crate::node_interface::Result<String> {
        self.submit_transaction(tx)
    }
}

impl SignTransaction for NodeInterface {
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

impl WalletDataSource for NodeInterface {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>> {
        self.unspent_boxes()
    }
}

pub fn new_node_interface() -> NodeInterface {
    NodeInterface::new(&get_node_api_key(), &get_node_ip(), &get_node_port())
}

/// Registers a scan with the node and either returns the `scan_id` or an error
pub fn register_scan(scan_json: &serde_json::Value) -> Result<ScanID> {
    let scan_json_t = json::parse(&serde_json::to_string(scan_json).unwrap()).unwrap();
    new_node_interface().register_scan(&scan_json_t)
}

/// Acquires unspent boxes from the node wallet
pub fn get_unspent_wallet_boxes() -> Result<Vec<ErgoBox>> {
    new_node_interface().unspent_boxes()
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet
pub fn get_highest_value_unspent_box() -> Result<ErgoBox> {
    new_node_interface().highest_value_unspent_box()
}

pub fn unspent_boxes_with_min_total(total: u64) -> Result<Vec<ErgoBox>> {
    new_node_interface().unspent_boxes_with_min_total(total)
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet and serializes it
pub fn get_serialized_highest_value_unspent_box() -> Result<String> {
    new_node_interface().serialized_highest_value_unspent_box()
}

/// Using the `scan_id` of a registered scan, acquires unspent boxes which have been found by said scan
pub fn get_scan_boxes(scan_id: &String) -> Result<Vec<ErgoBox>> {
    let res = new_node_interface().scan_boxes(scan_id);
    debug!("Scan boxes: {:?}", res);
    res
}

pub fn rescan_from_height(height: u32) -> Result<()> {
    new_node_interface().send_post_req(
        "/wallet/rescan",
        format!("{{ \"fromHeight\": {} }} ", height),
    )?;
    Ok(())
}

/// Get the current block height of the chain
pub fn current_block_height() -> Result<BlockHeight> {
    new_node_interface().current_block_height()
}

pub fn get_wallet_status() -> Result<WalletStatus> {
    new_node_interface().wallet_status()
}

// /// Sign an `UnsignedTransaction`.
// pub fn sign_transaction(unsigned_tx: &UnsignedTransaction) -> Result<Transaction> {
//     new_node_interface().sign_transaction(unsigned_tx)
// }

/// Submit a `Transaction` to the mempool.
pub fn submit_transaction(signed_tx: &Transaction) -> Result<TxId> {
    new_node_interface().submit_transaction(signed_tx)
}

/// Sign an `UnsignedTransaction` and then submit it to the mempool.
pub fn sign_and_submit_transaction(unsigned_tx: &UnsignedTransaction) -> Result<TxId> {
    let node = new_node_interface();
    log::debug!("Signing transaction: {:?}", unsigned_tx);
    let signed_tx = node.sign_transaction(unsigned_tx, None, None)?;
    log::debug!("Submitting signed transaction: {:?}", signed_tx);
    node.submit_transaction(&signed_tx)
}

pub fn assert_wallet_unlocked(node: &NodeInterface) {
    let unlocked = node.wallet_status().unwrap().unlocked;
    if !unlocked {
        error!("Wallet must be unlocked for node operations");
        std::process::exit(exitcode::SOFTWARE);
    } else {
        debug!("Wallet unlocked");
    }
}
