use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::TxId;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::scanning::NodeError;
use ergo_node_interface::NodeInterface;
use ergo_node_interface::ScanId;
use log::info;
use reqwest::Url;
use serde_json::json;
use thiserror::Error;

use crate::scans::ScanID;
use crate::wallet::WalletDataError;
use crate::wallet::WalletDataSource;

pub struct NodeApi {
    pub node: NodeInterface,
}

impl NodeApi {
    pub fn new(api_key: String, node_url: &Url) -> Self {
        let node = NodeInterface::from_url(&api_key, node_url.clone());
        Self { node }
    }

    pub fn get_change_address(&self) -> Result<NetworkAddress, NodeApiError> {
        let change_address_str = self
            .node
            .wallet_status()?
            .change_address
            .ok_or(NodeApiError::NoChangeAddressSetInNode)?;
        let addr = AddressEncoder::unchecked_parse_network_address_from_str(&change_address_str)?;
        Ok(addr)
    }

    /// Registers a scan with the node and either returns the `scan_id` or an error
    pub fn register_scan_raw(&self, scan_json: serde_json::Value) -> Result<ScanID, NodeApiError> {
        let scan_id = self.node.register_scan(scan_json)?;
        Ok(scan_id.to_string())
    }

    pub fn register_scan(
        &self,
        name: String,
        tracking_rule: serde_json::Value,
    ) -> std::result::Result<ScanId, NodeApiError> {
        let scan_json = json!({
            "scanName": name,
            "trackingRule": tracking_rule,
        });
        log::info!(
            "Registering Scan:\n{}",
            serde_json::to_string_pretty(&scan_json).unwrap()
        );
        let scan_id_str = self.register_scan_raw(scan_json)?;
        let scan_id_raw = scan_id_str
            .parse::<u64>()
            .map_err(|_| NodeApiError::InvalidScanId(scan_id_str))?;
        let scan_id = scan_id_raw.into();
        info!("Scan Successfully registered.\nID: {}", scan_id);
        Ok(scan_id)
    }

    pub fn deregister_scan(&self, scan_id: ScanId) -> Result<ScanId, NodeApiError> {
        log::info!("Deregistering Scan: {}", scan_id);
        let scan_id = self.node.deregister_scan(scan_id)?;
        Ok(scan_id)
    }

    pub fn rescan_from_height(&self, height: u32) -> Result<(), NodeApiError> {
        log::info!("Triggering wallet rescan");
        self.node.send_post_req(
            "/wallet/rescan",
            format!("{{ \"fromHeight\": {} }} ", height),
        )?;
        Ok(())
    }

    /// Sign an `UnsignedTransaction` and then submit it to the mempool.
    pub fn sign_and_submit_transaction(
        &self,
        unsigned_tx: &UnsignedTransaction,
    ) -> Result<TxId, NodeApiError> {
        log::trace!(
            "Signing transaction: {}",
            serde_json::to_string_pretty(&unsigned_tx).unwrap()
        );
        let signed_tx = self.node.sign_transaction(unsigned_tx, None, None)?;
        log::trace!(
            "Submitting signed transaction: {}",
            serde_json::to_string_pretty(&signed_tx).unwrap()
        );
        Ok(self.node.submit_transaction(&signed_tx)?)
    }
}

impl WalletDataSource for NodeApi {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, WalletDataError> {
        self.node.unspent_boxes().map_err(Into::into)
    }

    fn get_change_address(&self) -> Result<NetworkAddress, WalletDataError> {
        self.get_change_address().map_err(Into::into)
    }
}

#[derive(Debug, Error)]
pub enum NodeApiError {
    #[error("Node error: {0}")]
    NodeInterfaceError(#[from] NodeError),
    #[error("AddressEncoder error: {0}")]
    AddressEncoderError(#[from] AddressEncoderError),
    #[error("no change address is set in node")]
    NoChangeAddressSetInNode,
    #[error("invalid scan id: {0}")]
    InvalidScanId(String),
}
