#[macro_use]
extern crate json;

mod actions;
mod api;
mod encoding;
mod node_interface;
mod oracle_config;
mod oracle_state;
mod scans;
mod templates;

use std::thread;

pub type NanoErg = u64;
pub type BlockHeight = u64;
/// The id of the oracle pool epoch box
pub type EpochID = String;

fn main() {
    let op = oracle_state::OraclePool::new();

    thread::Builder::new()
        .name("Oracle Core API Thread".to_string())
        .spawn(|| {
            api::start_api();
        })
        .ok();

    loop {
        println!("{:?}", op.get_pool_deposits_state());
        println!("{:?}", op.get_datapoint_state());
        println!("{:?}", op.get_live_epoch_state());
        println!("{:?}", op.get_preparation_state());
    }

    // op.action_commit_datapoint(2389);
    // op.action_collect_funds();
    // op.action_start_next_epoch();
    // op.action_create_new_epoch();
    // op.action_collect_datapoints();
}
