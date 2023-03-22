use std::path::PathBuf;

use crate::address_util::{address_to_raw_for_register, AddressUtilError};
use crate::box_kind::{PoolBoxWrapperInputs, RefreshBoxWrapperInputs};
use crate::contracts::pool::PoolContractError;
use crate::contracts::refresh::RefreshContractError;
use crate::node_interface::node_api::{NodeApi, NodeApiError};
use crate::oracle_config::ORACLE_CONFIG;
use crate::spec_token::{BallotTokenId, OracleTokenId, UpdateTokenId};

use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::NodeError;
use json::JsonValue;
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

pub fn load_scan_ids() -> Result<JsonValue, anyhow::Error> {
    let path = get_scans_file_path();
    log::debug!("Loading scan IDs from {}", path.display());
    Ok(json::parse(&std::fs::read_to_string(path)?)?)
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

/// This function registers scanning for all of the pools oracles' Datapoint boxes for datapoint collection
pub fn register_datapoint_scan(
    oracle_pool_participant_token: &OracleTokenId,
) -> std::result::Result<Scan, ScanError> {
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": oracle_pool_participant_token.clone(),
        }
    ]
    } );

    Scan::register("All Datapoints Scan", scan_json)
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
