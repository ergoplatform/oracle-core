#[macro_use]
extern crate json;

mod actions;
mod encoding;
mod oracle_config;
mod oracle_state;
mod node_interface;
mod scans;
mod templates;


pub type NanoErg = u64;
pub type BlockHeight = u64;
/// The id of the oracle pool epoch box
pub type EpochID = String;




fn main() {
    println!("Hello, oracle pool!");

    let node_url = oracle_config::get_node_url();
    let node_api_key = oracle_config::get_node_api_key();

    let op = oracle_state::OraclePool::new();

    // op.get_pool_deposits_state();
    // op.get_datapoint_state();
    // op.get_preparation_state();
    op.get_live_epoch_state();

    // Go from P2PK or P2S address to tree encoded for use in register
    let a = node_interface::address_to_tree(&"3sSMhchmak6PHo5BXXavCiw1XkwZfsjox5N3sjVqbWVNyHojHJWBUL1JPswsEHttCiFfmVfwpFqaqjFF68D35vC9rkjCwwCFdKNR1GYdpTVbBdumH".to_string());
    let b = encoding::serialize_string(&a.unwrap());

    // Serialize a box id/string for use in a register
    let c = encoding::serialize_string(&"ddf651608e7324a641bb019f5fd4216e932b1521293b795892e50350fe581eed".to_string());

    op.action_commit_datapoint(23);
}



