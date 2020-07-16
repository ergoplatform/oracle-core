#[macro_use]
extern crate json;

mod actions;
mod encoding;
mod node_interface;
mod oracle_config;
mod oracle_state;
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

    println!("{:?}", op.get_pool_deposits_state());
    println!("{:?}", op.get_datapoint_state());
    println!("{:?}", op.get_live_epoch_state());
    println!("{:?}", op.get_preparation_state());

    // op.action_commit_datapoint(123489);
    // op.action_collect_funds();
    // op.action_start_next_epoch();
    // op.action_create_new_epoch();
    op.action_collect_datapoints();
}
