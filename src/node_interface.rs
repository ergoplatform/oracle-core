use crate::oracle_config::{get_node_api_header, get_node_url};
use crate::scans::ScanID;
use crate::{BlockHeight, P2PKAddress, P2SAddress, TxId};
use json::JsonValue;
use reqwest::blocking::{RequestBuilder, Response};
use reqwest::header::CONTENT_TYPE;
use serde_json::from_str;
use sigma_tree::chain::ErgoBox;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, NodeError>;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("The configured node is unreachable. Please ensure your config is correctly filled out and the node is running.")]
    NodeUnreachable,
    #[error("Failed reading response from node.")]
    FailedParsingNodeResponse,
    #[error("Failed reading response from node.")]
    NoBoxesFound,
    #[error("The node rejected the request you provided.\nNode Response: {0}")]
    BadRequest(String),
}

/// Registers a scan with the node and returns the `scan_id`
pub fn register_scan(scan_json: &JsonValue) -> Result<ScanID> {
    let endpoint = "/scan/register";
    let body = scan_json.clone().to_string();
    let res = send_post_req(endpoint, body);
    let res_json = parse_response_to_json(res)?;

    Ok(res_json["scanId"].to_string().clone())
}

/// Acquires unspent boxes from the node wallet
pub fn get_unspent_wallet_boxes() -> Result<Vec<ErgoBox>> {
    let endpoint = "/wallet/boxes/unspent?minConfirmations=0&minInclusionHeight=0";
    let res = send_get_req(endpoint);
    let res_json = parse_response_to_json(res)?;

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
    Ok(box_list)
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet
pub fn get_highest_value_unspent_box() -> Result<ErgoBox> {
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
            return Ok(b);
        }
    }
    Err(NodeError::NoBoxesFound)
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet and serializes it
pub fn get_serialized_highest_value_unspent_box() -> Result<String> {
    let ergs_box_id: String = get_highest_value_unspent_box()?.box_id().into();
    serialized_box_from_id(&ergs_box_id)
}

/// Using the `scan_id` of a registered scan, acquires unspent boxes which have been found by said scan
pub fn get_scan_boxes(scan_id: &String) -> Result<Vec<ErgoBox>> {
    let endpoint = "/scan/unspentBoxes/".to_string() + scan_id;
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    let mut box_list = vec![];
    for i in 0.. {
        let box_json = &res_json[i]["box"];
        if box_json.is_null() {
            break;
        } else {
            let res_ergo_box = from_str(&box_json.to_string());
            if let Ok(ergo_box) = res_ergo_box {
                box_list.push(ergo_box);
            } else {
                return Err(NodeError::FailedParsingNodeResponse);
            }
        }
    }
    Ok(box_list)
}

/// Generates (and sends) a tx using the node endpoints.
/// Input must be a json formatted request with rawInputs (and rawDataInputs)
/// manually selected or will be automatically selected by wallet.
/// Returns the resulting `TxId`.
pub fn send_transaction(tx_request_json: &JsonValue) -> Result<TxId> {
    let endpoint = "/wallet/transaction/send";
    let body = json::stringify(tx_request_json.clone());
    let res = send_post_req(endpoint, body);

    // println!("{:?}", tx_request_json.dump());

    let res_json = parse_response_to_json(res)?;
    let error_details = res_json["detail"].to_string().clone();

    // Check if send tx request failed and returned error json
    if error_details != "null" {
        return Err(NodeError::BadRequest(error_details));
    }
    // Otherwise if tx is valid and is posted, return just the tx id
    else {
        // Clean string to be only the tx_id value
        let tx_id = res_json
            .dump()
            .chars()
            .filter(|&c| c == '\\' || c == '\"')
            .collect();
        println!("Send Tx Result: {:?}", tx_id);

        return Ok(tx_id);
    }
}

/// Given a P2S Ergo address, extract the hex-encoded serialized ErgoTree (script)
pub fn address_to_tree(address: &P2SAddress) -> Result<String> {
    let endpoint = "/script/addressToTree/".to_string() + address;
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    Ok(res_json["tree"].to_string().clone())
}

/// Given a P2S Ergo address, convert it to a hex-encoded Sigma byte array constant
pub fn address_to_bytes(address: &P2SAddress) -> Result<String> {
    let endpoint = "/script/addressToBytes/".to_string() + address;
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    Ok(res_json["bytes"].to_string().clone())
}

/// Given an Ergo P2PK Address, convert it to a raw hex-encoded EC point
pub fn address_to_raw(address: &P2PKAddress) -> Result<String> {
    let endpoint = "/utils/addressToRaw/".to_string() + address;
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    Ok(res_json["raw"].to_string().clone())
}

/// Given an Ergo P2PK Address, convert it to a raw hex-encoded EC point
/// and prepend the type bytes so it is encoded and ready
/// to be used in a register.
pub fn address_to_raw_for_register(address: &P2PKAddress) -> Result<String> {
    let add = address_to_raw(address)?;
    Ok("07".to_string() + &add)
}

/// Given a raw hex-encoded EC point, convert it to a P2PK address
pub fn raw_to_address(raw: &String) -> Result<P2PKAddress> {
    let endpoint = "/utils/rawToAddress/".to_string() + raw;
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    Ok(res_json["address"].to_string().clone())
}

/// Given a raw hex-encoded EC point from a register (thus with type encoded characters in front),
/// convert it to a P2PK address
pub fn raw_from_register_to_address(typed_raw: &String) -> Result<P2PKAddress> {
    Ok(raw_to_address(&typed_raw[2..].to_string())?)
}

/// Given a `Vec<ErgoBox>` return the given boxes (which must be part of the UTXO-set) as
/// a vec of serialized strings in Base16 encoding
pub fn serialize_boxes(b: &Vec<ErgoBox>) -> Result<Vec<String>> {
    Ok(b.iter()
        .map(|b| serialized_box_from_id(&b.box_id().into()).unwrap_or("".to_string()))
        .collect())
}

/// Given an `ErgoBox` return the given box (which must be part of the UTXO-set) as
/// a serialized string in Base16 encoding
pub fn serialize_box(b: &ErgoBox) -> Result<String> {
    serialized_box_from_id(&b.box_id().into())
}

/// Given a box id return the given box (which must be part of the UTXO-set) as
/// a serialized string in Base16 encoding
pub fn serialized_box_from_id(box_id: &String) -> Result<String> {
    let endpoint = "/utxo/byIdBinary/".to_string() + box_id;
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    Ok(res_json["bytes"].to_string().clone())
}

/// Get the current block height of the chain
pub fn current_block_height() -> Result<BlockHeight> {
    let endpoint = "/info";
    let res = send_get_req(&endpoint);
    let res_json = parse_response_to_json(res)?;

    res_json["fullHeight"]
        .to_string()
        .parse()
        .map_err(|_| NodeError::FailedParsingNodeResponse)
}

/// Sets required headers for a request
fn set_req_headers(rb: RequestBuilder) -> RequestBuilder {
    rb.header("accept", "application/json")
        .header("api_key", get_node_api_header())
        .header(CONTENT_TYPE, "application/json")
}

/// Sends a GET request to the Ergo node
fn send_get_req(endpoint: &str) -> Result<Response> {
    let url = get_node_url().to_owned() + endpoint;
    let client = reqwest::blocking::Client::new().get(&url);
    set_req_headers(client)
        .send()
        .map_err(|_| NodeError::NodeUnreachable)
}

/// Sends a POST request to the Ergo node
fn send_post_req(endpoint: &str, body: String) -> Result<Response> {
    let url = get_node_url().to_owned() + endpoint;
    let client = reqwest::blocking::Client::new().post(&url);
    set_req_headers(client)
        .body(body)
        .send()
        .map_err(|_| NodeError::NodeUnreachable)
}

/// Parses response from node into JSON
fn parse_response_to_json(resp: Result<Response>) -> Result<JsonValue> {
    let json = resp?
        .text()
        .map(|t| json::parse(&t))
        .map_err(|_| NodeError::FailedParsingNodeResponse)?
        .map_err(|_| NodeError::FailedParsingNodeResponse)?;
    Ok(json)
}
