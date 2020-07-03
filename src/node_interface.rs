use crate::oracle_config::{get_node_api_key, get_node_url};
use crate::{BlockHeight, EpochID, NanoErg};
use json::{JsonValue};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use serde_json::{from_str};
use sigma_tree::chain::{ErgoBox};
use sigma_tree::ast::{Constant, ConstantVal};

/// Registers a scan with the node and returns the `scan_id`
pub fn register_scan(scan_json: &JsonValue) -> Option<String> {
    println!("{}", scan_json);
    let endpoint = get_node_url().to_owned() + "/scan/register";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let scan_json_string = json::stringify(scan_json.clone());
    let mut res = client
        .post(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .body(scan_json_string)
        .send()
        .ok()?;

    let result = res.text().ok()?;
    println!("{}", &result);
    let res_json = json::parse(&result).ok()?;
    Some(res_json["scanId"].to_string().clone())
}

/// Using the `scan_id` of a registered scan, acquires unspent boxes which have been found by said scan
pub fn get_scan_boxes(scan_id: &String) -> Option<Vec<ErgoBox>> {
    let endpoint = get_node_url().to_owned() + "/scan/unspentBoxes/" + scan_id;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let mut res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .ok()?;

    let res_json = json::parse(&res.text().ok()?).ok()?;
    let mut box_list = vec![];

    for i in 0.. {
        let box_json = &res_json[i]["box"];
        if box_json.is_null() {
            break;
        }
        else {
            if let Some(ergo_box) = from_str(&box_json.to_string()).ok() {
                box_list.push(ergo_box);
            }
        }
    }

    Some(box_list)
}


/// Generates (and sends) a tx using the node endpoints.
/// Input must be a json formatted request with either inputs (and data-inputs)
/// manualy selected or will be automatically selected by wallet.
pub fn send_transaction(tx_request_json: &JsonValue) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/wallet/transaction/send/";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let transaction_send_json = object! {
        requests: [
            tx_request_json.clone()
        ]
    };
    println!("{}", transaction_send_json.to_string());

    let mut res = client
        .post(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .body(transaction_send_json.to_string())
        .send()
        .ok()?;

    let result = res.text().ok()?;
    println!("Send Tx Result: {}", result);
    Some(result)
}


/// Given an Ergo address, extract the hex-encoded serialized ErgoTree (script)
/// which can then be utilized for many use cases 
/// (ie. comparing proposition bytes for scanning boxes)
pub fn address_to_tree(address: &String) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/script/addressToTree/" + address;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let mut res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .ok()?;

    let result = res.text().ok()?;
    let res_json = json::parse(&result).ok()?;
    Some(res_json["tree"].to_string().clone())
}

/// Given a box id that is being tracked by the node wallet
/// return the given box serialized in Base16 encoding
pub fn serialized_box_from_id(box_id: &String) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/utxo/byIdBinary/" + box_id;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let mut res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .ok()?;

    let result = res.text().ok()?;
    let res_json = json::parse(&result).ok()?;
    Some(res_json["bytes"].to_string().clone())
}

/// Get the current block height of the chain
/// To Be Implemented
pub fn current_block_height() -> Option<BlockHeight> {
    let endpoint = get_node_url().to_owned() + "/info";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let mut res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .ok()?;

    let result = res.text().ok()?;
    let res_json = json::parse(&result).ok()?;
    let blockheight = res_json["fullHeight"].to_string().parse().ok()?;
    Some(blockheight)
}

/// Gets a list of all addresses from the local unlocked node wallet
pub fn get_wallet_addresses() -> Option<Vec<String>> {
    let endpoint = get_node_url().to_owned() + "/wallet/addresses";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let mut res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .ok()?;

    let mut addresses: Vec<String> = vec![];
    for segment in res.text().ok()?.split("\"") {
        let seg = segment.trim();
        if seg.chars().next().unwrap() == '9' {
            addresses.push(seg.to_string());
        }
    }
    if addresses.len() == 0 {
        panic!("No addresses were found. Please make sure the node is running on the node-ip & node-port specified in `oracle-config.yaml` file and that your wallet is unlocked.");
    }
    Some(addresses)
}