#[macro_use]
extern crate json;

mod oracle_config;
mod node_interface;
mod oracle_state;
mod scans;

pub type NanoErg = u64;
pub type BlockHeight = u64;
pub type EpochID = String;




fn main() {
    println!("Hello, oracle pool!");

    let node_url = oracle_config::get_node_url();
    let node_api_key = oracle_config::get_node_api_key();

    let op = oracle_state::OraclePool::new();

}



