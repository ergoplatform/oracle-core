use crate::oracle_config::{get_node_api_key, get_node_url};
use crate::BlockHeight;
use json::JsonValue;
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use serde_json::from_str;
use sigma_tree::chain::ErgoBox;

/// Registers a scan with the node and returns the `scan_id`
pub fn register_scan(scan_json: &JsonValue) -> Option<String> {
    println!("{}", scan_json);
    let endpoint = get_node_url().to_owned() + "/scan/register";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .post(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .body(json::stringify(scan_json.clone()))
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let result = res.text().ok()?;
    println!("{}", &result);
    let res_json = json::parse(&result).ok()?;
    Some(res_json["scanId"].to_string().clone())
}

/// Acquires unspent boxes from the node wallet
pub fn get_unspent_wallet_boxes() -> Option<Vec<ErgoBox>> {
    let endpoint =
        get_node_url().to_owned() + "/wallet/boxes/unspent?minConfirmations=0&minInclusionHeight=0";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let res_json = json::parse(&res.text().ok()?).ok()?;
    let mut box_list = vec![];

    for i in 0.. {
        let box_json = &res_json[i]["box"];
        if box_json.is_null() {
            break;
        } else {
            if let Some(ergo_box) = from_str(&box_json.to_string()).ok() {
                box_list.push(ergo_box);
            }
        }
    }
    Some(box_list)
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet
pub fn get_highest_value_unspent_box() -> Option<ErgoBox> {
    let boxes = get_unspent_wallet_boxes()?;

    // Find the highest value amount held in a single box in the wallet
    let highest_value = boxes.iter().fold(0, |acc, b| {
        if b.value.value() > acc {
            b.value.value()
        } else {
            acc
        }
    });

    for b in boxes {
        if b.value.value() == highest_value {
            return Some(b);
        }
    }
    None
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet and serializes it
pub fn get_serialized_highest_value_unspent_box() -> Option<String> {
    let ergs_box_id: String = get_highest_value_unspent_box()?.box_id().into();
    serialized_box_from_id(&ergs_box_id)
}

/// Using the `scan_id` of a registered scan, acquires unspent boxes which have been found by said scan
pub fn get_scan_boxes(scan_id: &String) -> Option<Vec<ErgoBox>> {
    let endpoint = get_node_url().to_owned() + "/scan/unspentBoxes/" + scan_id;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let res_json = json::parse(&res.text().ok()?).ok()?;
    let mut box_list = vec![];

    for i in 0.. {
        let box_json = &res_json[i]["box"];
        if box_json.is_null() {
            break;
        } else {
            if let Some(ergo_box) = from_str(&box_json.to_string()).ok() {
                box_list.push(ergo_box);
            }
        }
    }
    Some(box_list)
}

/// Generates (and sends) a tx using the node endpoints.
/// Input must be a json formatted request with rawInputs (and rawDataInputs)
/// manually selected or will be automatically selected by wallet.
pub fn send_transaction(tx_request_json: &JsonValue) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/wallet/transaction/send";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;

    let res = client
        .post(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .body(json::stringify(tx_request_json.clone()))
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let result = res.text().ok()?;
    println!("Send Tx Result: {}", result);
    Some(result)
}

/// Given an Ergo address, extract the hex-encoded serialized ErgoTree (script)
pub fn address_to_tree(address: &String) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/script/addressToTree/" + address;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let result = res.text().ok()?;
    let res_json = json::parse(&result).ok()?;
    Some(res_json["tree"].to_string().clone())
}

/// Given an Ergo address, convert it to a hex-encoded Sigma byte array constant
///  which contains script bytes. Can then be utilized for many use cases
/// (ie. comparing proposition bytes for scanning boxes)
pub fn address_to_bytes(address: &String) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/script/addressToBytes/" + address;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let result = res.text().ok()?;
    let res_json = json::parse(&result).ok()?;
    Some(res_json["bytes"].to_string().clone())
}

/// Given a `Vec<ErgoBox>` return the given boxes (which must be part of the UTXO-set) as
/// a vec of serialized strings in Base16 encoding
pub fn serialize_boxes(b: &Vec<ErgoBox>) -> Option<Vec<String>> {
    Some(
        b.iter()
            .map(|b| serialized_box_from_id(&b.box_id().into()).unwrap_or("".to_string()))
            .collect(),
    )
}

/// Given an `ErgoBox` return the given box (which must be part of the UTXO-set) as
/// a serialized string in Base16 encoding
pub fn serialize_box(b: &ErgoBox) -> Option<String> {
    serialized_box_from_id(&b.box_id().into())
}

/// Given a box id return the given box (which must be part of the UTXO-set) as
/// a serialized string in Base16 encoding
pub fn serialized_box_from_id(box_id: &String) -> Option<String> {
    let endpoint = get_node_url().to_owned() + "/utxo/byIdBinary/" + box_id;
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

    let result = res.text().ok()?;
    let res_json = json::parse(&result).ok()?;
    Some(res_json["bytes"].to_string().clone())
}

/// Get the current block height of the chain
pub fn current_block_height() -> Option<BlockHeight> {
    let endpoint = get_node_url().to_owned() + "/info";
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).ok()?;
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

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
    let res = client
        .get(&endpoint)
        .header("accept", "application/json")
        .header("api_key", hapi_key)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .expect(
            "Ensure that your node is running, configured properly, and the wallet is unlocked.",
        );

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
