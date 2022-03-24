// Coding conventions
#![allow(dead_code)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::unit_arg)]
#![forbid(unsafe_code)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![deny(unused_imports)]
#![deny(clippy::wildcard_enum_match_arm)]
// #![deny(clippy::todo)]
#![deny(clippy::unimplemented)]

#[macro_use]
extern crate json;

mod actions;
mod api;
mod commands;
mod node_interface;
mod oracle_config;
mod oracle_state;
mod scans;
mod state;
mod templates;
mod wallet;

use actions::execute_action;
use anyhow::Error;
use commands::build_action;
use crossbeam::channel::bounded;
use log::info;
use node_interface::current_block_height;
use node_interface::get_wallet_change_address;
use oracle_config::PoolParameters;
use state::process;
use state::PoolState;
use std::env;
use std::thread;
use std::time::Duration;
use wallet::WalletData;

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
    simple_logging::log_to_file("oracle-core.log", log::LevelFilter::Info).ok();
    log_panics::init();
    let args: Vec<String> = env::args().collect();
    let (repost_sender, repost_receiver) = bounded(1);

    // Start Oracle Core GET API Server
    thread::Builder::new()
        .name("Oracle Core GET API Thread".to_string())
        .spawn(|| {
            api::start_get_api(repost_receiver);
        })
        .ok();

    // Start Oracle Core POST API Server
    thread::Builder::new()
        .name("Oracle Core POST API Thread".to_string())
        .spawn(|| {
            api::start_post_api();
        })
        .ok();

    let is_readonly = args.len() > 1 && &args[1] == "--readonly";
    loop {
        if let Err(e) = main_loop_iteration(is_readonly) {
            todo!()
        }
        // Delay loop restart
        thread::sleep(Duration::new(30, 0));
    }
}

fn main_loop_iteration(is_readonly: bool) -> Result<()> {
    let op = oracle_state::OraclePool::new();
    let parameters = oracle_config::PoolParameters::new();
    let height = current_block_height()?;
    let wallet = WalletData {};
    let change_address = get_wallet_change_address()?;
    // TODO: extract the check from print_into()
    // Check if properly synced.
    if let Err(e) = print_info(op.clone(), height, &parameters) {
        let mess = format!(
            "\nThe UTXO-Set scans have not found all of the oracle pool boxes yet.\n\nError: {:?}",
            e
        );
        print_and_log(&mess);
    }

    // If in `read only` mode
    if is_readonly {
        print_and_log("\n===============\nREAD ONLY MODE\n===============\nThe oracle core is running in `read only` mode.\nThis means that no transactions will be created and posted by the core.\nThis mode is intended to be used for easily reading the current state of the oracle pool protocol.");
    } else {
        // TODO: bootstrap should be initiated via command line option (or made manually by other means)
        // find out the current state of the pool
        let pool_state = match op.get_live_epoch_state() {
            Ok(live_epoch_state) => PoolState::LiveEpoch(live_epoch_state),
            Err(_) => PoolState::NeedsBootstrap,
        };
        if let Some(cmd) = process(pool_state, height)? {
            let action = build_action(
                cmd,
                op.live_epoch_stage.clone(),
                op.datapoint_stage.clone(),
                wallet,
                height as u32,
                change_address,
            )?;
            execute_action(action)?;
        }
    }
    Ok(())
}

/// Prints The Results Of An Action, Whether It Failed/Succeeded
pub fn print_action_results(action_res: &Result<String>, action_name: &str) {
    if let Ok(tx_id) = action_res {
        print_successful_action(action_name, tx_id);
    } else if let Err(e) = action_res {
        print_failed_action(action_name, e);
    }
}

/// Prints A Failed Action Message
fn print_failed_action(action_name: &str, error: &Error) {
    let message = format!(
        "Failed To Issue `{}` Transaction.\nError: {:?}",
        action_name, error
    );
    print_action_response(&message);
}

/// Prints A Successful Action Message
fn print_successful_action(action_name: &str, tx_id: &str) {
    let message = format!(
        "`{}` Transaction Has Been Posted.\nTransaction Id: {}",
        action_name, tx_id
    );
    print_action_response(&message);
}

/// Prints A Message With `---`s added
fn print_action_response(message: &str) {
    let mess = format!(        "--------------------------------------------------\n{}\n--------------------------------------------------",
        message
);
    print_and_log(&mess);
}

/// Prints And Logs Information About The State Of The Protocol
fn print_info(
    op: oracle_state::OraclePool,
    height: BlockHeight,
    parameters: &PoolParameters,
) -> Result<bool> {
    // Clear screen
    print!("\x1B[2J\x1B[1;1H");

    let datapoint_state = op.get_datapoint_state()?;
    let deposits_state = op.get_pool_deposits_state()?;
    let res_prep_state = op.get_preparation_state();
    let res_live_state = op.get_live_epoch_state();

    let mut info_string = ORACLE_CORE_ASCII.to_string();

    info_string.push_str("========================================================\n");
    info_string.push_str(&format!("Current Blockheight: {}\n", height));
    info_string.push_str(&format!("Current Tx Base Fee: {}\n", parameters.base_fee));
    info_string.push_str(&format!(
        "Pool Posting Schedule: {} Blocks\n",
        parameters.live_epoch_length + parameters.epoch_preparation_length
    ));
    info_string.push_str(&format!("Oracle Pool NFT ID: {}", op.oracle_pool_nft));

    info_string.push_str("\n========================================================\n");
    info_string.push_str(&format!("Pool Deposits State\n--------------------\nNumber Of Deposit Boxes: {}\nTotal nanoErgs In Deposit Boxes: {}\n", deposits_state.number_of_boxes, deposits_state.total_nanoergs));

    if let Ok(prep_state) = res_prep_state {
        info_string.push_str(&format!("\nEpoch Preparation State\n------------------------\nTotal Pool Funds: {}\nLatest Pool Datapoint: {}\nNext Epoch Ends: {}\n",
            prep_state.funds, prep_state.latest_pool_datapoint, prep_state.next_epoch_ends
        ));
    } else if let Ok(live_state) = res_live_state {
        info_string.push_str(&format!("\nLive Epoch State\n-----------------\nTotal Pool Funds: {}\nLatest Pool Datapoint: {}\nLive Epoch ID: {}\nCommit Datapoint In Live Epoch: {}\nLive Epoch Ends: {}\n",
            live_state.funds, live_state.latest_pool_datapoint, live_state.epoch_id, live_state.commit_datapoint_in_epoch, live_state.epoch_ends
        ));
    } else {
        info_string.push_str("Failed to find Epoch Preparation Box or Live Epoch Box.");
        info_string.push_str("\n========================================================\n");
    }

    info_string.push_str(&format!("\nOracle Datapoint State\n--------------------\nYour Latest Datapoint: {}\nDatapoint Origin Epoch ID: {}\nSubmitted At: {}", datapoint_state.datapoint, datapoint_state.origin_epoch_id, datapoint_state.creation_height
        ));
    info_string.push_str("\n========================================================\n");

    // Prints and logs the info String
    print_and_log(&info_string);

    Ok(true)
}

// Prints and logs a given message
pub fn print_and_log(message: &str) {
    println!("{}", message);
    info!("{}", message);
}
