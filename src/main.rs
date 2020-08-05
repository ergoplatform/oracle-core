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

use anyhow::Error;
use node_interface::current_block_height;
use std::thread;
use std::time::Duration;

/// A Base58 encoded String of a Ergo P2PK address. Using this type def until sigma-rust matures further with the actual Address type.
pub type P2PKAddress = String;
/// A Base58 encoded String of a Ergo P2S address. Using this type def until sigma-rust matures further with the actual Address type.
pub type P2SAddress = String;
/// Transaction ID
pub type TxId = String;
/// The smallest unit of the Erg currency.
pub type NanoErg = u64;
/// A block height of the chain.
pub type BlockHeight = u64;
/// Duration in number of blocks.
pub type BlockDuration = u64;
/// The id of the oracle pool epoch box
pub type EpochID = String;
/// A Base58 encoded String of a Token ID.
pub type TokenID = String;
// Anyhow Error used for the base Result return type.
pub type Result<T> = std::result::Result<T, anyhow::Error>;

static ORACLE_CORE_ASCII: &str = r#"
   ____                 _         _____
  / __ \               | |       / ____|
 | |  | |_ __ __ _  ___| | ___  | |     ___  _ __ ___
 | |  | | '__/ _` |/ __| |/ _ \ | |    / _ \| '__/ _ \
 | |__| | | | (_| | (__| |  __/ | |___| (_) | | |  __/
  \____/|_|  \__,_|\___|_|\___|  \_____\___/|_|  \___|
"#;

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
        let height = current_block_height().unwrap_or(0);
        print_info(op.clone(), height);

        let res_datapoint_state = op.get_datapoint_state();
        let res_prep_state = op.get_preparation_state();
        let res_live_state = op.get_live_epoch_state();
        let res_deposits_state = op.get_pool_deposits_state();

        // If the pool is in the Epoch Preparation stage
        if let Ok(prep_state) = res_prep_state {
            // Check state of pool deposit boxes
            if let Ok(deposits_state) = res_deposits_state {
                // Collect funds if sufficient funds exist worth collecting
                if deposits_state.total_nanoergs > 10000000 {
                    let action_res = op.action_collect_funds();
                    if let Ok(_) = action_res {
                        println!("-----\n`Collect Funds` Transaction Has Been Posted.\n-----");
                    } else if let Err(e) = action_res {
                        println!("-----\nFailed To Issue `Collect Funds` Transaction.\nError: {:?}\n-----", e);
                    }
                }
            }

            // Check epoch prep state
            let is_funded = prep_state.funds > parameters.max_pool_payout();
            let epoch_prep_over =
                height > prep_state.next_epoch_ends - parameters.live_epoch_length;
            let live_epoch_over = height >= prep_state.next_epoch_ends;

            // Check if height is prior to next epoch expected end
            // height and that the pool is funded.
            if epoch_prep_over && !live_epoch_over && is_funded {
                // Attempt to issue tx
                let action_res = op.action_start_next_epoch();
                if let Ok(_) = action_res {
                    println!("-----\n`Start Next Epoch` Transaction Has Been Posted.\n-----");
                } else if let Err(e) = action_res {
                    println!("-----\nFailed To Issue `Start Next Epoch` Transaction.\nError: {:?}\n-----", e);
                }
            }

            // Check if height is past the next epoch expected end
            // height and that the pool is funded.
            if live_epoch_over && is_funded {
                // Attempt to issue tx
                let action_res = op.action_create_new_epoch();
                if let Ok(_) = action_res {
                    println!("-----\n`Create New Epoch` Transaction Has Been Posted.\n-----");
                } else if let Err(e) = action_res {
                    println!("-----\nFailed To Issue `Create New Epoch` Transaction.\nError: {:?}\n-----", e);
                }
            }
        }

        // If the pool is in the Live Epoch stage
        if let Ok(epoch_state) = res_live_state {
            // Auto posting datapoint for testing protocol.
            // Delete later & replace with API datapoint submission.
            // if !epoch_state.commit_datapoint_in_epoch {
            //     let action_res = op.action_commit_datapoint(572321);
            //     if let Ok(_) = action_res {
            //         println!("-----\n`Commit Datapoint` Transaction Has Been Posted.\n-----");
            //     } else if let Err(e) = action_res {
            //         println!("-----\nFailed To Issue `Commit Datapoint` Transaction.\nError: {:?}\n-----", e);
            //     }
            // }

            // Check for opportunity to Collect Datapoints
            if height >= epoch_state.epoch_ends {
                let action_res = op.action_collect_datapoints();
                if let Ok(tx_id) = action_res {
                    println!("-----\n`Collect Datapoints` Transaction Has Been Posted.\n-----");
                } else if let Err(e) = action_res {
                    println!("-----\nFailed To Issue `Collect Datapoints` Transaction.\nError: {:?}\n-----", e);
                }
            }
        }

        // Delay loop restart
        thread::sleep(Duration::new(30, 0));
    }
}

/// Prints A Failed Action Message
fn print_failed_action(action_name: &String, error: &Error) {
    let message = format!(
        "Failed To Issue `{}` Transaction.\nError: {:?}",
        action_name, error
    );
    print_action_response(&message);
}

/// Prints A Successful Action Message
fn print_successful_action(action_name: &String, tx_id: &String) {
    let message = format!(
        "`{}` Transaction Has Been Posted.\nTransaction Id: {}",
        action_name, tx_id
    );
    print_action_response(&message);
}

/// Prints A Message With `---`s added
fn print_action_response(message: &String) {
    println!("-----\n{}\n-----", message);
}

/// Prints Information About The State Of The Protocol
fn print_info(op: oracle_state::OraclePool, height: BlockHeight) -> Result<bool> {
    // Clear screen
    print!("\x1B[2J\x1B[1;1H");

    println!("{}", ORACLE_CORE_ASCII);
    println!("========================================================");
    println!("Current Blockheight: {}", height);

    let datapoint_state = op.get_datapoint_state()?;
    let deposits_state = op.get_pool_deposits_state()?;
    let res_prep_state = op.get_preparation_state();
    let res_live_state = op.get_live_epoch_state();

    println!("========================================================");
    println!("Pool Deposits State\n--------------------\nNumber Of Deposit Boxes: {}\nTotal nanoErgs In Deposit Boxes: {}\n", deposits_state.number_of_boxes, deposits_state.total_nanoergs);
    if let Ok(prep_state) = res_prep_state {
        println!(
            "Epoch Preparation State\n------------------------\nTotal Pool Funds: {}\nNext Epoch Ends: {}\nLatest Pool Datapoint: {}\n",
            prep_state.funds, prep_state.next_epoch_ends, prep_state.latest_pool_datapoint
        );
    } else if let Ok(live_state) = res_live_state {
        println!(
            "Live Epoch State\n-----------------\nTotal Pool Funds: {}\nLive Epoch Ends: {}\nLatest Pool Datapoint: {}\nLive Epoch ID: {}\nCommit Datapoint In Live Epoch: {}\n",
            live_state.funds, live_state.epoch_ends, live_state.latest_pool_datapoint, live_state.epoch_id, live_state.commit_datapoint_in_epoch
        );
    }
    println!("Oracle Datapoint State\n--------------------\nYour Latest Datapoint: {}\nDatapoint Origin Epoch ID: {}\nSubmitted At: {}", datapoint_state.datapoint, datapoint_state.origin_epoch_id, datapoint_state.creation_height);
    println!("========================================================\n\n");
    Ok(true)
}
