use std::path::PathBuf;

use crate::address_util::{address_to_raw_for_register, AddressUtilError};
use crate::box_kind::{PoolBoxWrapperInputs, RefreshBoxWrapperInputs};
use crate::contracts::pool::PoolContractError;
use crate::contracts::refresh::RefreshContractError;
use crate::node_interface::node_api::{NodeApi, NodeApiError};
use crate::oracle_config::ORACLE_CONFIG;
use crate::pool_config::POOL_CONFIG;
use crate::spec_token::{BallotTokenId, OracleTokenId, UpdateTokenId};

use derive_more::{Display, From, Into};
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use log::info;
use once_cell::sync;
use serde_json::json;
use thiserror::Error;

/// Integer which is provided by the Ergo node to reference a given scan.
pub type ScanID = String;

#[derive(Debug, From, Error)]
pub enum ScanError {
    #[error("node error: {0}")]
    NodeError(NodeError),
    #[error("node api error: {0}")]
    NodeApiError(NodeApiError),
    #[error("no boxes found")]
    NoBoxesFound,
    #[error("failed to register scan")]
    FailedToRegister,
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("refresh contract error: {0}")]
    RefreshContract(RefreshContractError),
    #[error("pool contract error: {0}")]
    PoolContract(PoolContractError),
    #[error("address util error: {0}")]
    AddressUtilError(AddressUtilError),
}

#[derive(Debug, Copy, Clone, From, Into, Display)]
pub struct ScanId(u64);

#[derive(Debug, Clone, Copy)]
pub struct OracleTokenScan {
    id: ScanId,
}

impl OracleTokenScan {
    pub const NAME: &'static str = "All Datapoints Scan";

    pub fn load_from_json(json: &serde_json::Value) -> Result<Self, ScanError> {
        let id = json.get(Self::NAME).unwrap().as_u64().unwrap();
        Ok(OracleTokenScan { id: ScanId(id) })
    }

    pub fn tracking_rule(oracle_token_id: &OracleTokenId) -> serde_json::Value {
        json!({
        "predicate": "and",
        "args":
            [
                {
                    "predicate": "containsAsset",
                    "assetId": oracle_token_id,
                }
            ]
          })
    }

    pub fn register(
        node_api: &NodeApi,
        oracle_token_id: &OracleTokenId,
    ) -> Result<Self, ScanError> {
        let id = node_api.register_scan2(Self::NAME, Self::tracking_rule(oracle_token_id))?;
        Ok(OracleTokenScan { id })
    }

    pub fn get_old_scan(&self) -> Scan {
        Scan::new(Self::NAME, &self.id.0.to_string())
    }
}

impl ScanGetId for OracleTokenScan {
    fn get_scan_id(&self) -> ScanId {
        self.id
    }
}

pub trait ScanGetId {
    fn get_scan_id(&self) -> ScanId;
}

pub trait ScanGetBoxes: ScanGetId {
    fn get_boxes(&self) -> Result<Vec<ErgoBox>, ScanError> {
        let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
        let boxes = node_api
            .node
            .scan_boxes(&self.get_scan_id().0.to_string())?;
        Ok(boxes)
    }
}

impl ScanGetBoxes for OracleTokenScan {}

/// A `Scan` is a name + scan_id for a given scan with extra methods for acquiring boxes.
#[derive(Debug, Clone)]
pub struct Scan {
    name: &'static str,
    id: ScanID,
}

impl Scan {
    /// Create a new `Scan` with provided name & scan_id
    pub fn new(name: &'static str, scan_id: &String) -> Scan {
        Scan {
            name,
            id: scan_id.clone(),
        }
    }

    /// Registers a scan in the node and returns a `Scan` as a result
    pub fn register(
        name: &'static str,
        tracking_rule: serde_json::Value,
    ) -> std::result::Result<Scan, ScanError> {
        let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
        let scan_json = json!({
            "scanName": name,
            "trackingRule": tracking_rule,
        });

        info!(
            "Registering Scan:\n{}",
            serde_json::to_string_pretty(&scan_json).unwrap()
        );

        let scan_id = node_api.register_scan(&scan_json)?;
        info!("Scan Successfully Set.\nID: {}", scan_id);

        Ok(Scan::new(name, &scan_id))
    }

    /// Returns all boxes found by the scan
    pub fn get_boxes(&self) -> std::result::Result<Vec<ErgoBox>, ScanError> {
        let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
        let boxes = node_api.node.scan_boxes(&self.id)?;
        Ok(boxes)
    }

    /// Returns the first box found by the scan
    pub fn get_box(&self) -> std::result::Result<Option<ErgoBox>, ScanError> {
        Ok(self.get_boxes()?.first().cloned())
    }
}

pub static SCANS_DIR_PATH: sync::OnceCell<PathBuf> = sync::OnceCell::new();

pub fn get_scans_file_path() -> PathBuf {
    SCANS_DIR_PATH.get().unwrap().join("scanIDs.json")
}

/// Saves UTXO-set scans (specifically id) to scanIDs.json
pub fn save_scan_ids(scans: Vec<Scan>) -> std::result::Result<(), ScanError> {
    let mut id_json = json!({});
    for scan in scans {
        if &scan.id == "null" {
            return Err(ScanError::FailedToRegister);
        }
        id_json[scan.name] = scan.id.into();
    }
    let path = get_scans_file_path();
    log::debug!("Saving scan IDs to {}", path.display());
    std::fs::write(path, serde_json::to_string_pretty(&id_json).unwrap())?;
    Ok(())
}

pub fn load_scan_ids() -> Result<serde_json::Value, anyhow::Error> {
    let path = get_scans_file_path();
    log::debug!("Loading scan IDs from {}", path.display());
    let str = &std::fs::read_to_string(path)?;
    let json = serde_json::from_str(str)?;
    Ok(json)
}

/// This function registers scanning for the pool box
pub fn register_pool_box_scan(
    inputs: PoolBoxWrapperInputs,
) -> std::result::Result<Scan, ScanError> {
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": inputs.pool_nft_token_id.clone(),
        }
    ]
    } );

    Scan::register("Pool Box Scan", scan_json)
}

/// This function registers scanning for the refresh box
pub fn register_refresh_box_scan(
    scan_name: &'static str,
    inputs: RefreshBoxWrapperInputs,
) -> std::result::Result<Scan, ScanError> {
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": inputs.refresh_nft_token_id.clone(),
        }
    ]
    } );

    Scan::register(scan_name, scan_json)
}

/// This function registers scanning for the oracle's personal Datapoint box
pub fn register_local_oracle_datapoint_scan(
    oracle_pool_participant_token: &OracleTokenId,
    oracle_address: &NetworkAddress,
) -> std::result::Result<Scan, ScanError> {
    let oracle_add_bytes = address_to_raw_for_register(&oracle_address.to_base58())?;
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": oracle_pool_participant_token.clone(),
        },
                {
            "predicate": "equals",
            "register": "R4",
            "value": oracle_add_bytes.clone(),
        }
    ]
    } );

    Scan::register("Local Oracle Datapoint Scan", scan_json)
}

/// This function registers scanning for the local ballot box
pub fn register_local_ballot_box_scan(
    ballot_token_id: &BallotTokenId,
    ballot_token_owner_address: &NetworkAddress,
) -> std::result::Result<Scan, ScanError> {
    let ballot_add_bytes = address_to_raw_for_register(&ballot_token_owner_address.to_base58())?;
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": ballot_token_id.clone(),
        },
        {
            "predicate": "equals",
            "register": "R4",
            "value": ballot_add_bytes.clone(),
        }
    ]
    } );

    Scan::register("Local Ballot Box Scan", scan_json)
}

/// Scan for all ballot boxes matching token id of oracle pool.
pub fn register_ballot_box_scan(
    ballot_token_id: &BallotTokenId,
) -> std::result::Result<Scan, ScanError> {
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": ballot_token_id.clone(),
        }
        ] });
    Scan::register("Ballot Box Scan", scan_json)
}

pub fn register_update_box_scan(
    update_nft_token_id: &UpdateTokenId,
) -> std::result::Result<Scan, ScanError> {
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": update_nft_token_id.clone(),
        },
        ] });
    Scan::register("Update Box Scan", scan_json)
}

/// Register scans and save in scanIDs.json (if it doesn't already exist), and wait for rescan to complete
pub fn register_and_save_scans(node_api: &NodeApi) -> std::result::Result<(), anyhow::Error> {
    // let config = &POOL_CONFIG;
    if load_scan_ids().is_err() {
        register_and_save_scans_inner(node_api)?;
    };

    let wallet_height = node_api.node.wallet_status()?.height;
    let block_height = node_api.node.current_block_height()?;
    if wallet_height == block_height {
        return Ok(());
    }
    loop {
        let wallet_height = node_api.node.wallet_status()?.height;
        let block_height = node_api.node.current_block_height()?;
        println!("Scanned {}/{} blocks", wallet_height, block_height);
        if wallet_height == block_height {
            println!("Wallet Scan Complete!");
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
}

/// Registers and saves scans to `scanIDs.json` as well as performing wallet rescanning.
///
/// WARNING: will overwrite existing `scanIDs.json`!
fn register_and_save_scans_inner(node_api: &NodeApi) -> std::result::Result<(), anyhow::Error> {
    let pool_config = &POOL_CONFIG;
    let oracle_config = &ORACLE_CONFIG;
    let local_oracle_address = oracle_config.oracle_address.clone();
    let oracle_pool_participant_token_id = pool_config.token_ids.oracle_token_id.clone();
    let refresh_box_scan_name = "Refresh Box Scan";
    let scans = vec![
        OracleTokenScan::register(node_api, &pool_config.token_ids.oracle_token_id)?.get_old_scan(),
        register_update_box_scan(&pool_config.token_ids.update_nft_token_id)?,
        register_pool_box_scan(pool_config.pool_box_wrapper_inputs.clone())?,
        register_refresh_box_scan(
            refresh_box_scan_name,
            pool_config.refresh_box_wrapper_inputs.clone(),
        )?,
        register_local_oracle_datapoint_scan(
            &oracle_pool_participant_token_id,
            &local_oracle_address,
        )?,
        register_local_ballot_box_scan(
            &pool_config.token_ids.ballot_token_id,
            &oracle_config.oracle_address,
        )?,
        register_ballot_box_scan(&pool_config.token_ids.ballot_token_id)?,
    ];

    log::info!("Registering UTXO-Set Scans");
    save_scan_ids(scans)?;
    log::info!("Triggering wallet rescan");
    node_api.rescan_from_height(0)?;
    Ok(())
}
