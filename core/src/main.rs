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
mod box_kind;
mod commands;
mod contracts;
mod datapoint_source;
mod node_interface;
mod oracle_config;
mod oracle_state;
mod scans;
mod state;
mod templates;
mod wallet;

use actions::execute_action;
use anyhow::anyhow;
use anyhow::Error;
use clap::Parser;
use commands::build_action;
use crossbeam::channel::bounded;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use log::info;
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::Appender;
use log4rs::config::Root;
use log4rs::Config;
use node_interface::current_block_height;
use node_interface::get_wallet_status;
use oracle_config::PoolParameters;
use state::process;
use state::PoolState;
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
/// The epoch counter
pub type EpochID = u32;
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

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long = "Run oracle core in read-only mode")]
    read_only: bool,
}

fn main() {
    log_setup();

    let args = Args::parse();
    let (_, repost_receiver) = bounded(1);

    // Start Oracle Core GET API Server
    thread::Builder::new()
        .name("Oracle Core GET API Thread".to_string())
        .spawn(|| {
            api::start_get_api(repost_receiver);
        })
        .ok();

    loop {
        if let Err(_e) = main_loop_iteration(&args) {
            todo!()
        }
        // Delay loop restart
        thread::sleep(Duration::new(30, 0));
    }
}

fn main_loop_iteration(args: &Args) -> Result<()> {
    let op = oracle_state::OraclePool::new()?;
    let parameters = oracle_config::PoolParameters::new();
    let height = current_block_height()?;
    let wallet = WalletData::new();
    let change_address_str = get_wallet_status()?
        .change_address
        .ok_or_else(|| anyhow!("failed to get wallet's change address (locked wallet?)"))?;
    let change_address =
        AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str(&change_address_str)?;
    // TODO: extract the check from print_into()
    // Check if properly synced.
    if let Err(e) = print_info(&op, height, &parameters) {
        let mess = format!(
            "\nThe UTXO-Set scans have not found all of the oracle pool boxes yet.\n\nError: {:?}",
            e
        );
        print_and_log(&mess);
    }

    // If in `read only` mode
    if args.read_only {
        print_and_log("\n===============\nREAD ONLY MODE\n===============\nThe oracle core is running in `read only` mode.\nThis means that no transactions will be created and posted by the core.\nThis mode is intended to be used for easily reading the current state of the oracle pool protocol.");
    } else {
        // TODO: bootstrap should be initiated via command line option (or made manually by other means)
        // find out the current state of the pool
        let pool_state = match op.get_live_epoch_state() {
            Ok(live_epoch_state) => PoolState::LiveEpoch(live_epoch_state),
            Err(_) => PoolState::NeedsBootstrap,
        };
        if let Some(cmd) = process(pool_state, &*op.data_point_source, height)? {
            let action = build_action(cmd, op, &wallet, height as u32, change_address)?;
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
    op: &oracle_state::OraclePool,
    height: BlockHeight,
    parameters: &PoolParameters,
) -> Result<bool> {
    // Clear screen
    print!("\x1B[2J\x1B[1;1H");

    let datapoint_state = op.get_datapoint_state()?;
    let deposits_state = op.get_pool_deposits_state()?;
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

    if let Ok(live_state) = res_live_state {
        info_string.push_str(&format!("\nLive Epoch State\n-----------------\nLatest Pool Datapoint: {}\nLive Epoch ID: {}\nCommit Datapoint In Live Epoch: {}\nLive Epoch Ends: {}\n",
            live_state.latest_pool_datapoint, live_state.epoch_id, live_state.commit_datapoint_in_epoch, live_state.epoch_ends
        ));
    } else {
        info_string.push_str("Failed to find Epoch Preparation Box or Live Epoch Box.");
        info_string.push_str("\n========================================================\n");
    }

    if let Some(datapoint_state) = datapoint_state {
        info_string.push_str(&format!("\nOracle Datapoint State\n--------------------\nYour Latest Datapoint: {}\nDatapoint Origin Epoch ID: {}\nSubmitted At: {}", datapoint_state.datapoint, datapoint_state.origin_epoch_id, datapoint_state.creation_height
        ));
    }
    info_string.push_str("\n========================================================\n");

    // Prints and logs the info String
    print_and_log(&info_string);

    Ok(true)
}

// Prints and logs a given message
// TODO: use info! directly instead
pub fn print_and_log(message: &str) {
    info!("{}", message);
}

fn log_setup() {
    let stdout = ConsoleAppender::builder().build();

    let logfile = FileAppender::builder().build("oracle-core.log").unwrap();

    // TODO: rotate log file
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .build(LevelFilter::Info),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    log_panics::init();
}
