#[macro_use]
extern crate json;

mod actions;
mod encoding;
mod oracle_config;
mod oracle_state;
mod node_interface;
mod scans;


pub type NanoErg = u64;
pub type BlockHeight = u64;
/// The id of the oracle pool epoch box
pub type EpochID = String;




fn main() {
    println!("Hello, oracle pool!");

    let node_url = oracle_config::get_node_url();
    let node_api_key = oracle_config::get_node_api_key();

    let op = oracle_state::OraclePool::new();

    op.get_pool_deposits_state();
    op.get_datapoint_state();

    // Go from P2PK or P2S address to tree encoded for use in register
    let a = node_interface::address_to_tree(&"3sSMhchmak6PHo5BXXavCiw1XkwZfsjox5N3sjVqbWVNyHojHJWBUL1JPswsEHttCiFfmVfwpFqaqjFF68D35vC9rkjCwwCFdKNR1GYdpTVbBdumH".to_string());
    let b = encoding::serialize_string(&a.unwrap());

    // Serialize a box id/string for use in a register
    let c = encoding::serialize_string(&"6690d6e27ac9d76785e9baf66c440a21b43492263fac6223bbe6488281273a8a".to_string());

}



