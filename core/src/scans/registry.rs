use std::path::PathBuf;

use crate::node_interface::node_api::NodeApi;
use crate::node_interface::node_api::NodeApiError;
use crate::pool_config::PoolConfig;
use crate::spec_token::BallotTokenId;
use crate::spec_token::BuybackTokenId;
use crate::spec_token::OracleTokenId;
use crate::spec_token::PoolTokenId;
use crate::spec_token::RefreshTokenId;
use crate::spec_token::UpdateTokenId;

use ::serde::Deserialize;
use ::serde::Serialize;
use once_cell::sync;
use proptest::strategy::NoShrink;
use thiserror::Error;

use super::generic_token_scan::GenericTokenScan;
use super::NodeScanId;
use super::ScanError;

pub static SCANS_DIR_PATH: sync::OnceCell<PathBuf> = sync::OnceCell::new();

pub fn get_scans_file_path() -> PathBuf {
    SCANS_DIR_PATH.get().unwrap().join("scanIDs.json")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeScanRegistry {
    #[serde(rename = "All Datapoints Scan")]
    pub oracle_token_scan: GenericTokenScan<OracleTokenId>,
    #[serde(rename = "Pool Box Scan")]
    pub pool_token_scan: GenericTokenScan<PoolTokenId>,
    #[serde(rename = "Ballot Box Scan")]
    pub ballot_token_scan: GenericTokenScan<BallotTokenId>,
    #[serde(rename = "Refresh Box Scan")]
    pub refresh_token_scan: GenericTokenScan<RefreshTokenId>,
    #[serde(rename = "Update Box Scan")]
    pub update_token_scan: GenericTokenScan<UpdateTokenId>,
    pub buyback_token_scan: Option<GenericTokenScan<BuybackTokenId>>,
}

impl NodeScanRegistry {
    fn load_from_json_str(json_str: &str) -> Result<Self, anyhow::Error> {
        Ok(serde_json::from_str(json_str)
            .map_err(|e| NodeScanRegistryError::Parse(e.to_string()))?)
    }

    fn save_to_json_str(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }

    fn save_to_json_file(&self, file_path: &PathBuf) -> Result<(), anyhow::Error> {
        let json_str = self.save_to_json_str();
        log::debug!("Saving scan IDs to {}", file_path.display());
        Ok(std::fs::write(file_path, json_str)
            .map_err(|e| NodeScanRegistryError::Io(e.to_string()))?)
    }

    fn register_and_save_scans_inner(
        node_api: &NodeApi,
        pool_config: &PoolConfig,
    ) -> std::result::Result<Self, anyhow::Error> {
        log::info!("Registering UTXO-Set Scans");
        let oracle_token_scan =
            GenericTokenScan::register(node_api, &pool_config.token_ids.oracle_token_id)?;
        let pool_token_scan =
            GenericTokenScan::register(node_api, &pool_config.token_ids.pool_nft_token_id)?;
        let ballot_token_scan =
            GenericTokenScan::register(node_api, &pool_config.token_ids.ballot_token_id)?;
        let refresh_token_scan =
            GenericTokenScan::register(node_api, &pool_config.token_ids.refresh_nft_token_id)?;
        let update_token_scan =
            GenericTokenScan::register(node_api, &pool_config.token_ids.update_nft_token_id)?;
        let buyback_token_scan = if let Some(buyback_token_id) = pool_config.buyback_token_id {
            Some(GenericTokenScan::register(node_api, &buyback_token_id)?)
        } else {
            None
        };
        let registry = Self {
            oracle_token_scan,
            pool_token_scan,
            ballot_token_scan,
            refresh_token_scan,
            update_token_scan,
            buyback_token_scan,
        };
        registry.save_to_json_file(&get_scans_file_path())?;
        node_api.rescan_from_height(0)?;
        Ok(registry)
    }

    pub fn load() -> Result<Self, anyhow::Error> {
        let path = get_scans_file_path();
        log::info!("Loading scan IDs from {}", path.display());
        let json_str =
            std::fs::read_to_string(path).map_err(|e| NodeScanRegistryError::Io(e.to_string()))?;
        let registry = Self::load_from_json_str(&json_str)?;
        Ok(registry)
    }

    pub fn ensure_node_registered_scans(
        node_api: &NodeApi,
        pool_config: &PoolConfig,
    ) -> std::result::Result<Self, anyhow::Error> {
        let path = get_scans_file_path();
        log::info!("Loading scan IDs from {}", path.display());
        let registry = if let Ok(json_str) = std::fs::read_to_string(path) {
            let registry = Self::load_from_json_str(&json_str)?;
            if let Some(buyback_token_id) = pool_config.buyback_token_id {
                if registry.buyback_token_scan.is_none() {
                    let buyback_token_scan =
                        GenericTokenScan::register(node_api, &buyback_token_id)?;
                    node_api.rescan_from_height(0)?;
                    let node_scan_registry = Self {
                        buyback_token_scan: Some(buyback_token_scan),
                        ..registry
                    };
                    node_scan_registry.save_to_json_file(&get_scans_file_path())?;
                    node_scan_registry
                } else {
                    registry
                }
            } else {
                registry
            }
        } else {
            log::info!("scans not found");
            Self::register_and_save_scans_inner(node_api, pool_config)?
        };
        wait_for_node_rescan(node_api)?;
        Ok(registry)
    }

    pub fn deregister_all_scans(self, node_api: &NodeApi) -> Result<(), NodeApiError> {
        node_api.deregister_scan(self.oracle_token_scan.scan_id())?;
        node_api.deregister_scan(self.pool_token_scan.scan_id())?;
        node_api.deregister_scan(self.ballot_token_scan.scan_id())?;
        node_api.deregister_scan(self.refresh_token_scan.scan_id())?;
        node_api.deregister_scan(self.update_token_scan.scan_id())?;
        Ok(())
    }
}

pub fn wait_for_node_rescan(node_api: &NodeApi) -> Result<(), NodeApiError> {
    let wallet_height = node_api.node.wallet_status()?.height;
    let block_height = node_api.node.current_block_height()?;
    if wallet_height == block_height {
        log::debug!("No wallet scan is running");
        return Ok(());
    }
    Ok(loop {
        let wallet_height = node_api.node.wallet_status()?.height;
        let block_height = node_api.node.current_block_height()?;
        println!("Scanned {}/{} blocks", wallet_height, block_height);
        if wallet_height == block_height {
            log::info!("Wallet Scan Complete!");
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    })
}

#[derive(Debug, Error)]
pub enum NodeScanRegistryError {
    #[error("Error registering scan: {0}")]
    Scan(#[from] ScanError),
    #[error("Error node: {0}")]
    NodeApi(#[from] NodeApiError),
    #[error("Error parsing scans file: {0}")]
    Parse(String),
    #[error("Error reading/writing file: {0}")]
    Io(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scans::NodeScanId;
    use ergo_node_interface::ScanId;
    use expect_test::expect;
    use pretty_assertions::assert_eq;

    fn expect_json(json_str: &str, expected_json: expect_test::Expect) {
        expected_json.assert_eq(json_str);
    }

    #[test]
    fn parse_legacy_json() {
        let json_str = r#"{ 
        "All Datapoints Scan": "185",
        "Update Box Scan": "186",
        "Pool Box Scan": "187",
        "Refresh Box Scan": "188",
        "Local Oracle Datapoint Scan": "189",
        "Local Ballot Box Scan": "190",
        "Ballot Box Scan": "191" 
        }"#;
        let registry = NodeScanRegistry::load_from_json_str(json_str).unwrap();
        assert_eq!(registry.oracle_token_scan.scan_id(), ScanId::from(185));
        assert_eq!(registry.pool_token_scan.scan_id(), ScanId::from(187));
    }

    #[test]
    fn check_encoded_json_id_as_string() {
        let registry = NodeScanRegistry {
            oracle_token_scan: GenericTokenScan::new(ScanId::from(185)),
            pool_token_scan: GenericTokenScan::new(ScanId::from(187)),
            ballot_token_scan: GenericTokenScan::new(ScanId::from(191)),
            refresh_token_scan: GenericTokenScan::new(ScanId::from(188)),
            update_token_scan: GenericTokenScan::new(ScanId::from(186)),
            buyback_token_scan: None,
        };
        let json_str = registry.save_to_json_str();
        expect_json(
            &json_str,
            expect![[r#"
                {
                  "All Datapoints Scan": "185",
                  "Pool Box Scan": "187",
                  "Ballot Box Scan": "191",
                  "Refresh Box Scan": "188",
                  "Update Box Scan": "186"
                }"#]],
        );
    }

    #[test]
    fn json_roundtrip() {
        let registry = NodeScanRegistry {
            oracle_token_scan: GenericTokenScan::new(ScanId::from(185)),
            pool_token_scan: GenericTokenScan::new(ScanId::from(187)),
            ballot_token_scan: GenericTokenScan::new(ScanId::from(191)),
            refresh_token_scan: GenericTokenScan::new(ScanId::from(188)),
            update_token_scan: GenericTokenScan::new(ScanId::from(186)),
            buyback_token_scan: None,
        };
        let json_str = registry.save_to_json_str();
        let registry2 = NodeScanRegistry::load_from_json_str(&json_str).unwrap();
        assert_eq!(registry, registry2);
    }
}
