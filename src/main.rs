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
use std::time::Duration;

pub type NanoErg = u64;
pub type BlockHeight = u64;
/// The id of the oracle pool epoch box
pub type EpochID = String;

use node_interface::current_block_height;

fn main() {
    let op = oracle_state::OraclePool::new();
    let parameters = oracle_config::PoolParameters::new();

    thread::Builder::new()
        .name("Oracle Core API Thread".to_string())
        .spawn(|| {
            api::start_api();
        })
        .ok();

    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        let height = current_block_height().unwrap_or(0);
        println!("Blockheight: {}", height);

        // If the pool is in the Epoch Preparation stage
        if let Some(prep_state) = op.get_preparation_state() {
            println!("{:?}", prep_state);

            // Check state of pool deposit boxes
            if let Some(deposits_state) = op.get_pool_deposits_state() {
                // Collect funds if sufficient funds exist worth collecting
                if deposits_state.total_nanoergs > 10000000 {
                    if let Some(_) = op.action_collect_funds() {
                        println!("-----\n`Collect Funds` Transaction Has Been Posted.\n-----");
                    } else {
                        println!("-----\nFailed To Issue `Collect Funds` Transaction.\n-----");
                    }
                }

                // Check epoch prep state
                if let Some(prep_state) = op.get_preparation_state() {
                    let is_funded = prep_state.funds > parameters.max_pool_payout();

                    // Check if height is prior to next epoch expected end
                    // height and that the pool is funded.
                    if height < prep_state.next_epoch_ends && is_funded {
                        // Attempt to issue tx
                        if let Some(_) = op.action_start_next_epoch() {
                            println!(
                                "-----\n`Start Next Epoch` Transaction Has Been Posted.\n-----"
                            );
                        } else {
                            println!(
                                "-----\nFailed To Issue `Start Next Epoch` Transaction.\n-----"
                            );
                        }
                    }

                    // Check if height is past the next epoch expected end
                    // height and that the pool is funded.
                    if height > prep_state.next_epoch_ends && is_funded {
                        // Attempt to issue tx
                        if let Some(_) = op.action_create_new_epoch() {
                            println!(
                                "-----\n`Create New Epoch` Transaction Has Been Posted.\n-----"
                            );
                        } else {
                            println!(
                                "-----\nFailed To Issue `Create New Epoch` Transaction.\n-----"
                            );
                        }
                    }
                }
            }
        }
        // If the pool is in the Live Epoch stage
        if let Some(epoch_state) = op.get_live_epoch_state() {
            println!("{:?}", epoch_state);

            // Auto posting datapoint for testing protocol.
            // Delete later & replace with API datapoint submission.
            if !epoch_state.commit_datapoint_in_epoch && height > epoch_state.epoch_ends - 2 {
                if let Some(_) = op.action_commit_datapoint(572321) {
                    println!("-----\n`Commit Datapoint` Transaction Has Been Posted.\n-----");
                } else {
                    println!("-----\nFailed To Issue `Commit Datapoint` Transaction.\n-----");
                }
            }

            // Check for opportunity to Collect Datapoints
            if height >= epoch_state.epoch_ends {
                // Attempt to collect datapoints
                if let Some(_) = op.action_collect_datapoints() {
                    println!("-----\n`Collect Datapoints` Transaction Has Been Posted.\n-----");
                } else {
                    println!("-----\nFailed To Issue `Collect Datapoints` Transaction.\n-----");
                }
            }
        }

        println!("{:?}", op.get_pool_deposits_state());
        println!("{:?}", op.get_datapoint_state());

        // Delay loop restart by 1 second
        thread::sleep(Duration::new(30, 0));
    }
}
