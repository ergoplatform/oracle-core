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
use clap::Parser;
use commands::build_action;
use crossbeam::channel::bounded;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::Appender;
use log4rs::config::Root;
use log4rs::Config;
use node_interface::current_block_height;
use node_interface::get_wallet_status;
use oracle_state::OraclePool;
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
    setup_log();
    // TODO: log parsed config parameters
    // TODO: log contract parameters

    let args = Args::parse();
    if args.read_only {
        println!("\n===============\nREAD ONLY MODE\n===============\nThe oracle core is running in `read only` mode.\nThis means that no transactions will be created and posted by the core.\nThis mode is intended to be used for easily reading the current state of the oracle pool protocol.");
    };
    let (_, repost_receiver) = bounded(1);

    // Start Oracle Core GET API Server
    thread::Builder::new()
        .name("Oracle Core GET API Thread".to_string())
        .spawn(|| {
            api::start_get_api(repost_receiver);
        })
        .ok();

    let op = OraclePool::new().unwrap();
    loop {
        if let Err(_e) = main_loop_iteration(args.read_only, &op) {
            // TODO: set exit code
            todo!()
        }
        // Delay loop restart
        thread::sleep(Duration::new(30, 0));
    }
}

fn main_loop_iteration(is_read_only: bool, op: &OraclePool) -> Result<()> {
    let height = current_block_height()?;
    let wallet = WalletData::new();
    let change_address_str = get_wallet_status()?
        .change_address
        .ok_or_else(|| anyhow!("failed to get wallet's change address (locked wallet?)"))?;
    let change_address =
        AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str(&change_address_str)?;
    let pool_state = match op.get_live_epoch_state() {
        Ok(live_epoch_state) => PoolState::LiveEpoch(live_epoch_state),
        Err(_) => PoolState::NeedsBootstrap,
    };
    if let Some(cmd) = process(pool_state, &*op.data_point_source, height)? {
        let action = build_action(cmd, op, &wallet, height as u32, change_address)?;
        if !is_read_only {
            execute_action(action)?;
        }
    }
    Ok(())
}

fn setup_log() {
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
                // TODO: read log level from environment variable or config file
                .build(LevelFilter::Info),
        )
        .unwrap();

    let _ = log4rs::init_config(config).unwrap();

    log_panics::init();
}
