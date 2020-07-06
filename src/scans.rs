/// This file holds logic related to UTXO-set scans
use crate::encoding::{serialize_string};
use crate::node_interface::{register_scan, address_to_tree};
use json;

/// Saves UTXO-set scan ids to scanIDs.json
pub fn save_scan_ids_locally(epoch_preparation_id: String, live_epoch_id: String, datapoint_id: String, pool_deposit_id: String) {
    let id_json = object!{
        epoch_preparation_scan_id: epoch_preparation_id,
        live_epoch_scan_id: live_epoch_id,
        datapoint_scan_id: datapoint_id,
        pool_deposit_scan_id: pool_deposit_id,
    };
    std::fs::write("scanIDs.json", json::stringify_pretty(id_json, 4)).expect("Unable to save UTXO-set scan ids to scanIDs.json");
}


/// This function registers scanning for the Epoch Preparation stage box
pub fn register_epoch_preparation_scan(oracle_pool_nft: &String, epoch_preparation_address: &String) -> String {

    // ErgoTree bytes of the P2S address/script
    let epoch_prep_bytes = address_to_tree(epoch_preparation_address).expect("Failed to access node to use addressToTree.");

    // Scan for NFT id + Epoch Preparation address
    let scan_json = object!{
        scanName: "Epoch Preparation Scan",
        trackingRule: {
            "predicate": "and",
            "args": [
                {
                "predicate": "containsAsset",
                "assetId": oracle_pool_nft.clone(),
                },
                {
                "predicate": "equals",
                "bytes": epoch_prep_bytes.clone(),
                }
            ]}
        };

    register_scan(&scan_json).expect("Failed to register epoch preparation scan.")
}


/// This function registers scanning for the Oracle Pool Epoch stage box
pub fn register_live_epoch_scan(oracle_pool_nft: &String, live_epoch_address: &String) -> String {

    // ErgoTree bytes of the P2S address/script
    let live_epoch_bytes = address_to_tree(live_epoch_address).expect("Failed to access node to use addressToTree.");

    // Scan for NFT id + Oracle Pool Epoch address
    let scan_json = object!{
        scanName: "Oracle Pool Epoch Scan",
        trackingRule: {
            "predicate": "and",
            "args": [
                {
                "predicate": "containsAsset",
                "assetId": oracle_pool_nft.clone(),
                },
                {
                "predicate": "equals",
                "bytes": live_epoch_bytes.clone(),
                }
            ]}
        };

    register_scan(&scan_json).expect("Failed to register oracle pool epoch scan.")
}

/// This function registers scanning for the oracle's personal Datapoint box
pub fn register_datapoint_scan(oracle_pool_participant_token: &String, datapoint_address: &String, oracle_address: &String) -> String {

    // ErgoTree bytes of the datapoint P2S address/script
    let datapoint_add_bytes = address_to_tree(datapoint_address).expect("Failed to access node to use addressToTree.");

    // ErgoTree bytes of the datapoint P2S address/script
    let oracle_add_bytes = serialize_string(&address_to_tree(oracle_address).expect("Failed to access node to use addressToTree."))[6..].to_string();
    println!("{}", oracle_add_bytes);

    // Scan for pool participant token id + datapoint contract address + oracle_address in R4
    let scan_json = object!{
        scanName: "Personal Oracle Datapoint Scan",
        trackingRule: {
            "predicate": "and",
            "args": [
                {
                "predicate": "containsAsset",
                "assetId": oracle_pool_participant_token.clone(),
                },
                {
                "predicate": "equals",
                "bytes": datapoint_add_bytes.clone(),
                },
                {
                "predicate": "equals",
                "register": "R4",
                "bytes": oracle_add_bytes.clone(),
                }
            ]}
        };

    register_scan(&scan_json).expect("Failed to register oracle datapoint scan.")
}

/// This function registers scanning for any boxes in the Pool Deposit stage address
pub fn register_pool_deposit_scan(pool_deposit_address: &String) -> String {

    // ErgoTree bytes of the datapoint P2S address/script
    let pool_dep_add_bytes = address_to_tree(pool_deposit_address).expect("Failed to access node to use addressToTree.");
    println!("Pool Dep Bytes: {}", pool_dep_add_bytes);

    // Scan for boxes at pool deposit address
    let scan_json = object!{
        scanName: "Oracle Pool Deposit Scan",
        trackingRule: {
                "predicate": "equals",
                "bytes": pool_dep_add_bytes.clone(),
        }
    };

    register_scan(&scan_json).expect("Failed to register pool deposit scan.")
}