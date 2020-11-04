use crate::oracle_config::{get_node_api_key, get_node_ip, get_node_port};
use ergo_lib::chain::ergo_box::ErgoBox;
use ergo_node_interface::node_interface::{NodeError, NodeInterface};
use ergo_offchain_utilities::{BlockHeight, P2PKAddressString, P2SAddressString, ScanID, TxId};
use json::JsonValue;

pub type Result<T> = std::result::Result<T, NodeError>;

pub fn new_node_interface() -> NodeInterface {
    NodeInterface::new(&get_node_api_key(), &get_node_ip(), &get_node_port())
}

/// Registers a scan with the node and either returns the `scan_id` or an error
pub fn register_scan(scan_json: &JsonValue) -> Result<ScanID> {
    new_node_interface().register_scan(scan_json)
}

/// Acquires unspent boxes from the node wallet
pub fn get_unspent_wallet_boxes() -> Result<Vec<ErgoBox>> {
    new_node_interface().unspent_boxes()
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet
pub fn get_highest_value_unspent_box() -> Result<ErgoBox> {
    new_node_interface().highest_value_unspent_box()
}

pub fn unspent_boxes_with_min_total(total: u64) -> Result<Vec<ErgoBox>> {
    new_node_interface().unspent_boxes_with_min_total(total)
}

pub fn serialized_unspent_boxes_with_min_total(total: u64) -> Result<Vec<String>> {
    new_node_interface().serialized_unspent_boxes_with_min_total(total)
}

/// Acquires the unspent box with the highest value of Ergs inside
/// from the wallet and serializes it
pub fn get_serialized_highest_value_unspent_box() -> Result<String> {
    new_node_interface().serialized_highest_value_unspent_box()
}

/// Using the `scan_id` of a registered scan, acquires unspent boxes which have been found by said scan
pub fn get_scan_boxes(scan_id: &String) -> Result<Vec<ErgoBox>> {
    new_node_interface().scan_boxes(scan_id)
}

/// Generates (and sends) a tx using the node endpoints.
/// Input must be a json formatted request with rawInputs (and rawDataInputs)
/// manually selected or will be automatically selected by wallet.
/// Returns the resulting `TxId`.
pub fn send_transaction(tx_request_json: &JsonValue) -> Result<TxId> {
    new_node_interface().generate_and_submit_transaction(&tx_request_json.dump())
}

/// Given a P2S Ergo address, extract the hex-encoded serialized ErgoTree (script)
pub fn address_to_tree(address: &P2SAddressString) -> Result<String> {
    new_node_interface().p2s_to_tree(address)
}

/// Given a P2S Ergo address, convert it to a hex-encoded Sigma byte array constant
pub fn address_to_bytes(address: &P2SAddressString) -> Result<String> {
    new_node_interface().p2s_to_bytes(address)
}

/// Given an Ergo P2PK Address, convert it to a raw hex-encoded EC point
pub fn address_to_raw(address: &P2PKAddressString) -> Result<String> {
    new_node_interface().p2pk_to_raw(address)
}

/// Given an Ergo P2PK Address, convert it to a raw hex-encoded EC point
/// and prepend the type bytes so it is encoded and ready
/// to be used in a register.
pub fn address_to_raw_for_register(address: &P2PKAddressString) -> Result<String> {
    new_node_interface().p2pk_to_raw_for_register(address)
}

/// Given a raw hex-encoded EC point, convert it to a P2PK address
pub fn raw_to_address(raw: &String) -> Result<P2PKAddressString> {
    new_node_interface().raw_to_p2pk(raw)
}

/// Given a raw hex-encoded EC point from a register (thus with type encoded characters in front),
/// convert it to a P2PK address
pub fn raw_from_register_to_address(typed_raw: &String) -> Result<P2PKAddressString> {
    new_node_interface().raw_from_register_to_p2pk(typed_raw)
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
    new_node_interface().serialized_box_from_id(box_id)
}

/// Get the current block height of the chain
pub fn current_block_height() -> Result<BlockHeight> {
    new_node_interface().current_block_height()
}
