/// This file holds logic related to UTXO-set scans
use crate::node_interface::{
    address_to_bytes, address_to_raw_for_register, get_scan_boxes, register_scan, serialize_box,
    serialize_boxes,
};
use crate::Result;
use anyhow::anyhow;
use json;
use json::JsonValue;
use sigma_tree::chain::ErgoBox;

/// Integer which is provided by the Ergo node to reference a given scan.
pub type ScanID = String;

/// A `Scan` is a name + scan_id for a given scan with extra methods for acquiring boxes.
#[derive(Debug, Clone)]
pub struct Scan {
    name: String,
    id: ScanID,
}

impl Scan {
    /// Create a new `Scan` with provided name & scan_id
    pub fn new(name: &String, scan_id: &String) -> Scan {
        Scan {
            name: name.clone(),
            id: scan_id.clone(),
        }
    }

    /// Registers a scan in the node and returns a `Scan` as a result
    pub fn register(name: &String, trackingRule: JsonValue) -> Scan {
        let scan_json = object! {
        scanName: name.clone(),
        trackingRule: trackingRule,
        };
        let scan_id = register_scan(&scan_json).expect("Failed to register scan.");
        println!("Scan ID: {}", scan_id);
        Scan::new(name, &scan_id)
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
            .nth(0)
            .ok_or(anyhow!("No Boxes Found."))
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
pub fn save_scan_ids_locally(scans: Vec<Scan>) {
    let mut id_json = object! {};
    for scan in scans {
        id_json[scan.name] = scan.id.into();
    }
    std::fs::write("scanIDs.json", json::stringify_pretty(id_json, 4))
        .expect("Unable to save UTXO-set scan ids to scanIDs.json");
}

/// This function registers scanning for the Live Epoch stage box
pub fn register_live_epoch_scan(oracle_pool_nft: &String, live_epoch_address: &String) -> Scan {
    // ErgoTree bytes of the P2S address/script
    let live_epoch_bytes =
        address_to_bytes(live_epoch_address).expect("Failed to access node to use addressToBytes.");

    // Scan for NFT id + Oracle Pool Epoch address
    let scan_json = object! {
        "predicate": "and",
        "args": [
            {
            "predicate": "containsAsset",
            "assetId": oracle_pool_nft.clone(),
            },
            {
            "predicate": "equals",
            "value": live_epoch_bytes.clone(),
            }
        ]
    };

    Scan::register(&"Live Epoch Scan".to_string(), scan_json)
}

/// This function registers scanning for the Epoch Preparation stage box
pub fn register_epoch_preparation_scan(
    oracle_pool_nft: &String,
    epoch_preparation_address: &String,
) -> Scan {
    // ErgoTree bytes of the P2S address/script
    let epoch_prep_bytes = address_to_bytes(epoch_preparation_address)
        .expect("Failed to access node to use addressToBytes.");

    // Scan for NFT id + Epoch Preparation address
    let scan_json = object! {
        "predicate": "and",
        "args": [
            {
            "predicate": "containsAsset",
            "assetId": oracle_pool_nft.clone(),
            },
            {
            "predicate": "equals",
            "value": epoch_prep_bytes.clone(),
            }
        ]
    };

    Scan::register(&"Epoch Preparation Scan".to_string(), scan_json)
}

/// This function registers scanning for the oracle's personal Datapoint box
pub fn register_local_oracle_datapoint_scan(
    oracle_pool_participant_token: &String,
    datapoint_address: &String,
    oracle_address: &String,
) -> Scan {
    // ErgoTree bytes of the datapoint P2S address/script
    let datapoint_add_bytes =
        address_to_bytes(datapoint_address).expect("Failed to access node to use addressToBytes.");

    // Raw EC bytes + type identifier
    let oracle_add_bytes = address_to_raw_for_register(&oracle_address)
        .expect("Failed to access node to use addressToBytes.");

    // Scan for pool participant token id + datapoint contract address + oracle_address in R4
    let scan_json = object! {
        "predicate": "and",
        "args": [
            {
            "predicate": "containsAsset",
            "assetId": oracle_pool_participant_token.clone(),
            },
            {
            "predicate": "equals",
            "value": datapoint_add_bytes.clone(),
            },
            {
            "predicate": "equals",
            "register": "R4",
            "value": oracle_add_bytes.clone(),
            }
        ]
    };

    Scan::register(&"Local Oracle Datapoint Scan".to_string(), scan_json)
}

/// This function registers scanning for all of the pools oracles' Datapoint boxes for datapoint collection
pub fn register_datapoint_scan(
    oracle_pool_participant_token: &String,
    datapoint_address: &String,
) -> Scan {
    // ErgoTree bytes of the datapoint P2S address/script
    let datapoint_add_bytes =
        address_to_bytes(datapoint_address).expect("Failed to access node to use addressToBytes.");

    // Scan for pool participant token id + datapoint contract address + oracle_address in R4
    let scan_json = object! {
        "predicate": "and",
        "args": [
            {
            "predicate": "containsAsset",
            "assetId": oracle_pool_participant_token.clone(),
            },
            {
            "predicate": "equals",
            "value": datapoint_add_bytes.clone(),
            }
        ]
    };

    Scan::register(&"All Datapoints Scan".to_string(), scan_json)
}

/// This function registers scanning for any boxes in the Pool Deposit stage address
pub fn register_pool_deposit_scan(pool_deposit_address: &String) -> Scan {
    // ErgoTree bytes of the datapoint P2S address/script
    let pool_dep_add_bytes = address_to_bytes(pool_deposit_address)
        .expect("Failed to access node to use addressToBytes.");
    println!("Pool Dep Bytes: {}", pool_dep_add_bytes);

    // Scan for boxes at pool deposit address
    let scan_json = object! {
                "predicate": "equals",
                "value": pool_dep_add_bytes.clone(),
    };

    println!("{:?}", scan_json.dump());
    Scan::register(&"Pool Deposits Scan".to_string(), scan_json)
}
