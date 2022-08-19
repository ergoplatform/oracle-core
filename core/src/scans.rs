use crate::address_util::{address_to_raw_for_register, AddressUtilError};
use crate::box_kind::{PoolBoxWrapperInputs, RefreshBoxWrapperInputs};
use crate::contracts::pool::{PoolContract, PoolContractError};
use crate::contracts::refresh::{RefreshContract, RefreshContractError};
/// This file holds logic related to UTXO-set scans
use crate::node_interface::{get_scan_boxes, register_scan, serialize_box, serialize_boxes};

use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::Constant;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_node_interface::node_interface::NodeError;
use log::info;
use serde_json::json;
use thiserror::Error;

/// Integer which is provided by the Ergo node to reference a given scan.
pub type ScanID = String;

pub type Result<T> = std::result::Result<T, ScanError>;

#[derive(Debug, From, Error)]
pub enum ScanError {
    #[error("node error: {0}")]
    NodeError(NodeError),
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
    pub fn register(name: &'static str, tracking_rule: serde_json::Value) -> Result<Scan> {
        let scan_json = json!({
            "scanName": name,
            "trackingRule": tracking_rule,
        });

        info!(
            "Registering Scan:\n{}",
            serde_json::to_string_pretty(&scan_json).unwrap()
        );

        let scan_id = register_scan(&scan_json)?;
        info!("Scan Successfully Set.\nID: {}", scan_id);

        Ok(Scan::new(name, &scan_id))
    }

    /// Returns all boxes found by the scan
    pub fn get_boxes(&self) -> Result<Vec<ErgoBox>> {
        let boxes = get_scan_boxes(&self.id)?;
        Ok(boxes)
    }

    /// Returns the first box found by the scan
    pub fn get_box(&self) -> Result<ErgoBox> {
        self.get_boxes()?
            .into_iter()
            .next()
            .ok_or(ScanError::NoBoxesFound)
    }

    /// Returns all boxes found by the scan
    /// serialized and ready to be used as rawInputs
    pub fn get_serialized_boxes(&self) -> Result<Vec<String>> {
        let boxes = serialize_boxes(&self.get_boxes()?)?;
        Ok(boxes)
    }

    /// Returns the first box found by the registered scan
    /// serialized and ready to be used as a rawInput
    pub fn get_serialized_box(&self) -> Result<String> {
        let ser_box = serialize_box(&self.get_box()?)?;
        Ok(ser_box)
    }
}

/// Saves UTXO-set scans (specifically id) to scanIDs.json
pub fn save_scan_ids_locally(scans: Vec<Scan>) -> Result<bool> {
    let mut id_json = json!({});
    for scan in scans {
        if &scan.id == "null" {
            return Err(ScanError::FailedToRegister);
        }
        id_json[scan.name] = scan.id.into();
    }
    std::fs::write(
        "scanIDs.json",
        serde_json::to_string_pretty(&id_json).unwrap(),
    )?;
    Ok(true)
}

/// This function registers scanning for the pool box
pub fn register_pool_box_scan(inputs: PoolBoxWrapperInputs) -> Result<Scan> {
    // ErgoTree bytes of the P2S address/script
    let pool_box_tree_bytes = PoolContract::new(inputs.into())?
        .ergo_tree()
        .to_scan_bytes();

    // Scan for NFT id + Oracle Pool Epoch address
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": inputs.pool_nft_token_id.clone(),
        },
        {
            "predicate": "equals",
            "value": &pool_box_tree_bytes
        }
    ]
    } );

    Scan::register("Pool Box Scan", scan_json)
}

/// This function registers scanning for the refresh box
pub fn register_refresh_box_scan(
    scan_name: &'static str,
    inputs: RefreshBoxWrapperInputs,
) -> Result<Scan> {
    // ErgoTree bytes of the P2S address/script
    let tree_bytes = RefreshContract::load(inputs.into())?
        .ergo_tree()
        .to_scan_bytes();

    // Scan for NFT id + Oracle Pool Epoch address
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": inputs.refresh_nft_token_id.clone(),
        },
        {
            "predicate": "equals",
            "value": tree_bytes,
        }
    ]
    } );

    Scan::register(scan_name, scan_json)
}

/// This function registers scanning for the oracle's personal Datapoint box
pub fn register_local_oracle_datapoint_scan(
    oracle_pool_participant_token: &TokenId,
    datapoint_address: &ErgoTree,
    oracle_address: &NetworkAddress,
) -> Result<Scan> {
    // Raw EC bytes + type identifier
    let oracle_add_bytes = address_to_raw_for_register(&oracle_address.to_base58())?;
    let datapoint_bytes = datapoint_address.to_scan_bytes();

    // Scan for pool participant token id + datapoint contract address + oracle_address in R4
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": oracle_pool_participant_token.clone(),
        },
        {
            "predicate": "equals",
            "value": datapoint_bytes,
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
    oracle_pool_participant_token: &TokenId,
    datapoint_address: &ErgoTree,
) -> Result<Scan> {
    let datapoint_bytes = datapoint_address.to_scan_bytes();
    // Scan for pool participant token id + datapoint contract address + oracle_address in R4
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": oracle_pool_participant_token.clone(),
        },
        {
            "predicate": "equals",
            "value": datapoint_bytes,
        }
    ]
    } );

    Scan::register("All Datapoints Scan", scan_json)
}

/// This function registers scanning for the local ballot box
pub fn register_local_ballot_box_scan(
    ballot_contract_address: &ErgoTree,
    ballot_token_id: &TokenId,
    ballot_token_owner_address: &NetworkAddress,
) -> Result<Scan> {
    // Raw EC bytes + type identifier
    let ballot_add_bytes = address_to_raw_for_register(&ballot_token_owner_address.to_base58())?;
    let ballot_contract_bytes = ballot_contract_address.to_scan_bytes();
    // Scan for pool participant token id + datapoint contract address + oracle_address in R4
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": ballot_token_id.clone(),
        },
        {
            "predicate": "equals",
            "value": ballot_contract_bytes,
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

/// Scan for all ballot boxes matching token id of oracle pool. When updating the pool box only ballot boxes voting for the new pool will be spent
pub fn register_ballot_box_scan(
    ballot_contract_address: &ErgoTree,
    ballot_token_id: &TokenId,
) -> Result<Scan> {
    let scan_json = json! ( {
        "predicate": "and",
        "args": [
        {
            "predicate": "containsAsset",
            "assetId": ballot_token_id.clone(),
        },
        {
            "predicate": "equals",
            "value": ballot_contract_address.to_scan_bytes(),
        }
        ] });
    Scan::register("Ballot Box Scan", scan_json)
}

// TODO: We don't currently scan for ErgoTree, since config does not store min_votes
pub fn register_update_box_scan(update_nft_token_id: &TokenId) -> Result<Scan> {
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

/// Convert a chain type to Coll[Byte] for scans
pub trait ToScanBytes {
    fn to_scan_bytes(&self) -> String;
}

impl ToScanBytes for ErgoTree {
    fn to_scan_bytes(&self) -> String {
        base16::encode_lower(
            &Constant::from(self.sigma_serialize_bytes().unwrap())
                .sigma_serialize_bytes()
                .unwrap(),
        )
    }
}
