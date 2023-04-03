use crate::address_util::AddressUtilError;
use crate::box_kind::RefreshBoxWrapperInputs;
use crate::contracts::pool::PoolContractError;
use crate::contracts::refresh::RefreshContractError;
use crate::node_interface::node_api::{NodeApi, NodeApiError};
use crate::oracle_config::ORACLE_CONFIG;
use crate::spec_token::UpdateTokenId;

use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use ergo_node_interface::ScanId;
use log::info;
use serde_json::json;
use thiserror::Error;

mod generic_token_scan;
mod oracle_token_scan;
mod registry;

pub use generic_token_scan::*;
pub use oracle_token_scan::*;
pub use registry::*;

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

pub trait NodeScan: NodeScanId {
    fn scan_name(&self) -> &'static str;
    fn get_old_scan(&self) -> Scan {
        Scan::new(self.scan_name(), &u64::from(self.scan_id()).to_string())
    }
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

        let scan_id = node_api.register_scan_raw(scan_json)?;
        info!("Scan Successfully Set.\nID: {}", scan_id);

        Ok(Scan::new(name, &scan_id))
    }

    /// Returns all boxes found by the scan
    pub fn get_boxes(&self) -> std::result::Result<Vec<ErgoBox>, ScanError> {
        let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
        let scan_id: ScanId = self.id.parse::<u64>().unwrap().into();
        let boxes = node_api.node.scan_boxes(scan_id)?;
        Ok(boxes)
    }

    /// Returns the first box found by the scan
    pub fn get_box(&self) -> std::result::Result<Option<ErgoBox>, ScanError> {
        Ok(self.get_boxes()?.first().cloned())
    }
}

// /// Saves UTXO-set scans (specifically id) to scanIDs.json
// pub fn save_scan_ids(scans: Vec<Scan>) -> std::result::Result<(), ScanError> {
//     let mut id_json = json!({});
//     for scan in scans {
//         if &scan.id == "null" {
//             return Err(ScanError::FailedToRegister);
//         }
//         id_json[scan.name] = scan.id.into();
//     }
//     let path = get_scans_file_path();
//     log::debug!("Saving scan IDs to {}", path.display());
//     std::fs::write(path, serde_json::to_string_pretty(&id_json).unwrap())?;
//     Ok(())
// }

pub fn load_scan_ids() -> Result<serde_json::Value, anyhow::Error> {
    let path = get_scans_file_path();
    log::debug!("Loading scan IDs from {}", path.display());
    let str = &std::fs::read_to_string(path)?;
    let json = serde_json::from_str(str)?;
    Ok(json)
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

///// Register scans and save in scanIDs.json (if it doesn't already exist), and wait for rescan to complete
//pub fn register_and_save_scans(node_api: &NodeApi) -> std::result::Result<(), anyhow::Error> {
//    // let config = &POOL_CONFIG;
//    if load_scan_ids().is_err() {
//        register_and_save_scans_inner(node_api)?;
//    };

//    let wallet_height = node_api.node.wallet_status()?.height;
//    let block_height = node_api.node.current_block_height()?;
//    if wallet_height == block_height {
//        return Ok(());
//    }
//    loop {
//        let wallet_height = node_api.node.wallet_status()?.height;
//        let block_height = node_api.node.current_block_height()?;
//        println!("Scanned {}/{} blocks", wallet_height, block_height);
//        if wallet_height == block_height {
//            println!("Wallet Scan Complete!");
//            break;
//        }
//        std::thread::sleep(std::time::Duration::from_secs(1));
//    }
//    Ok(())
//}

///// Registers and saves scans to `scanIDs.json` as well as performing wallet rescanning.
/////
///// WARNING: will overwrite existing `scanIDs.json`!
//fn register_and_save_scans_inner(node_api: &NodeApi) -> std::result::Result<(), anyhow::Error> {
//    let pool_config = &POOL_CONFIG;
//    let oracle_config = &ORACLE_CONFIG;
//    let local_oracle_address = oracle_config.oracle_address.clone();
//    let oracle_pool_participant_token_id = pool_config.token_ids.oracle_token_id.clone();
//    let refresh_box_scan_name = "Refresh Box Scan";
//    let scans = vec![
//        GenericTokenScan::register(node_api, &pool_config.token_ids.oracle_token_id)?
//            .get_old_scan(),
//        register_update_box_scan(&pool_config.token_ids.update_nft_token_id)?,
//        GenericTokenScan::register(node_api, &pool_config.token_ids.pool_nft_token_id)?
//            .get_old_scan(),
//        register_refresh_box_scan(
//            refresh_box_scan_name,
//            pool_config.refresh_box_wrapper_inputs.clone(),
//        )?,
//        register_local_oracle_datapoint_scan(
//            &oracle_pool_participant_token_id,
//            &local_oracle_address,
//        )?,
//        register_local_ballot_box_scan(
//            &pool_config.token_ids.ballot_token_id,
//            &oracle_config.oracle_address,
//        )?,
//        register_ballot_box_scan(&pool_config.token_ids.ballot_token_id)?,
//    ];

//    log::info!("Registering UTXO-Set Scans");
//    save_scan_ids(scans)?;
//    log::info!("Triggering wallet rescan");
//    node_api.rescan_from_height(0)?;
//    Ok(())
//}
