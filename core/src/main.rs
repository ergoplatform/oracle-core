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
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]

#[macro_use]
extern crate lazy_static;

mod actions;
mod address_util;
mod api;
mod box_kind;
mod cli_commands;
mod contracts;
mod datapoint_source;
mod default_parameters;
mod explorer_api;
mod logging;
mod migrate;
mod node_interface;
mod oracle_config;
mod oracle_state;
mod oracle_types;
mod pool_commands;
mod pool_config;
mod scans;
mod serde;
mod spec_token;
mod state;
mod templates;
#[cfg(test)]
mod tests;
mod wallet;

use actions::PoolAction;
use anyhow::Context;
use clap::{Parser, Subcommand};
use crossbeam::channel::bounded;
use datapoint_source::RuntimeDataPointSource;
use ergo_lib::ergo_chain_types::Digest32;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use log::error;
use log::LevelFilter;
use node_interface::assert_wallet_unlocked;
use node_interface::node_api::NodeApi;
use oracle_config::ORACLE_CONFIG;
use oracle_state::register_and_save_scans;
use oracle_state::OraclePool;
use oracle_types::BlockHeight;
use pool_commands::build_action;
use pool_commands::publish_datapoint::PublishDatapointActionError;
use pool_commands::refresh::RefreshActionError;
use pool_commands::PoolCommandError;
use pool_config::DEFAULT_POOL_CONFIG_FILE_NAME;
use pool_config::POOL_CONFIG;
use scans::get_scans_file_path;
use state::process;
use state::PoolState;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::actions::execute_action;
use crate::api::start_rest_server;
use crate::default_parameters::print_contract_hashes;
use crate::migrate::check_migration_to_split_config;
use crate::oracle_config::OracleConfig;
use crate::oracle_config::DEFAULT_ORACLE_CONFIG_FILE_NAME;
use crate::oracle_config::ORACLE_CONFIG_FILE_PATH;
use crate::oracle_config::ORACLE_CONFIG_OPT;
use crate::pool_config::POOL_CONFIG_FILE_PATH;

const APP_VERSION: &str = concat!(
    "v",
    env!("CARGO_PKG_VERSION"),
    "+",
    env!("GIT_COMMIT_HASH"),
    " ",
    env!("GIT_COMMIT_DATE")
);

#[derive(Debug, Parser)]
#[clap(author, version = APP_VERSION, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
    /// Increase the logging verbosity
    #[clap(short, long)]
    verbose: bool,
    /// Set path of oracle configuration file to use. Default is ./oracle_config.yaml
    #[clap(long)]
    oracle_config_file: Option<String>,
    /// Set path of pool configuration file to use. Default is ./pool_config.yaml
    #[clap(long)]
    pool_config_file: Option<String>,
    /// Set folder path for the data files (scanIDs.json, logs). Default is the current folder.
    #[clap(short, long)]
    data_dir: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate oracle_config.yaml with default settings.
    GenerateOracleConfig,
    /// Bootstrap a new oracle-pool or generate a bootstrap config template file using default
    /// contract scripts and parameters.
    Bootstrap {
        /// The name of the bootstrap config file.
        yaml_config_name: String,
        #[clap(short, long)]
        /// Set this flag to output a bootstrap config template file to the given filename. If
        /// filename already exists, return error.
        generate_config_template: bool,
    },

    /// Run the oracle-pool
    Run {
        /// Run in read-only mode
        #[clap(long)]
        read_only: bool,
        #[clap(long)]
        /// Set this flag to enable the REST API. NOTE: SSL is not used!
        enable_rest_api: bool,
    },

    /// Send reward tokens accumulated in the oracle box to a chosen address
    ExtractRewardTokens {
        /// Base58 encoded address to send reward tokens to
        rewards_address: String,
    },

    /// Print the number of reward tokens earned by the oracle (in the last posted/collected oracle box)
    PrintRewardTokens,

    /// Transfer an oracle token to a chosen address.
    TransferOracleToken {
        /// Base58 encoded address to send oracle token to
        oracle_token_address: String,
    },

    /// Vote to update the oracle pool
    VoteUpdatePool {
        /// The base16-encoded blake2b hash of the serialized pool box contract for the new pool box.
        new_pool_box_address_hash_str: String,
        /// The base16-encoded reward token id of the new pool box (use existing if unchanged)
        reward_token_id_str: String,
        /// The reward token amount in the pool box at the time of update transaction is committed.
        reward_token_amount: u32,
        /// The creation height of the existing update box.
        update_box_creation_height: u32,
    },
    /// Initiate the Update Pool transaction.
    /// Run with no arguments to show diff between oracle_config.yaml and oracle_config_updated.yaml
    /// Updated config file must be created using --prepare-update command first
    UpdatePool {
        /// New pool box hash. Must match hash of updated pool contract
        new_pool_box_hash: Option<String>,
        /// New reward token id (optional, base64)
        reward_token_id: Option<String>,
        /// New reward token amount, required if new token id was voted for
        reward_token_amount: Option<u64>,
    },
    /// Prepare updating oracle pool with new contracts/parameters.
    /// Creates new refresh box and pool box if needed (e.g. if new reward tokens are minted)
    PrepareUpdate {
        /// Name of the parameters file (.yaml) with new contract parameters
        update_file: String,
    },

    /// Print base 64 encodings of the blake2b hash of ergo-tree bytes of each contract
    PrintContractHashes,

    ImportPoolUpdate {
        /// Name of the pool config file (.yaml) with new contract parameters
        pool_config_file: String,
    },
}

fn main() {
    let args = Args::parse();

    ORACLE_CONFIG_FILE_PATH
        .set(
            PathBuf::from_str(
                &args
                    .oracle_config_file
                    .unwrap_or_else(|| DEFAULT_ORACLE_CONFIG_FILE_NAME.to_string()),
            )
            .unwrap(),
        )
        .unwrap();
    POOL_CONFIG_FILE_PATH
        .set(
            PathBuf::from_str(
                &args
                    .pool_config_file
                    .unwrap_or_else(|| DEFAULT_POOL_CONFIG_FILE_NAME.to_string()),
            )
            .unwrap(),
        )
        .unwrap();

    let pool_config_path = POOL_CONFIG_FILE_PATH.get().unwrap();
    let oracle_config_path = ORACLE_CONFIG_FILE_PATH.get().unwrap();

    if !pool_config_path.exists() && oracle_config_path.exists() {
        if let Err(e) = check_migration_to_split_config(oracle_config_path, pool_config_path) {
            eprintln!("Failed to migrate to split config: {}", e);
        }
    }

    if !oracle_config_path.exists() {
        OracleConfig::write_default_config_file(oracle_config_path);
        println!(
            "{} not found. Default config file is generated.",
            oracle_config_path.display()
        );
        println!(
            "Please, set the required parameters(node credentials, oracle_address) and run again"
        );
        return;
    }

    let cmdline_log_level = if args.verbose {
        Some(LevelFilter::Debug)
    } else {
        None
    };
    let data_dir_path = if let Some(ref data_dir) = args.data_dir {
        Path::new(&data_dir).to_path_buf()
    } else {
        env::current_dir().unwrap()
    };

    let config_log_level = ORACLE_CONFIG_OPT
        .clone()
        .map(|c| c.log_level)
        .ok()
        .flatten();
    logging::setup_log(cmdline_log_level, config_log_level, &data_dir_path);

    scans::SCANS_DIR_PATH.set(data_dir_path).unwrap();

    let datapoint_source = RuntimeDataPointSource::new(
        POOL_CONFIG.data_point_source,
        ORACLE_CONFIG.data_point_source_custom_script.clone(),
    )
    .unwrap();

    let mut tokio_runtime = tokio::runtime::Runtime::new().unwrap();

    #[allow(clippy::wildcard_enum_match_arm)]
    match args.command {
        Command::GenerateOracleConfig => {
            if !oracle_config_path.exists() {
                OracleConfig::write_default_config_file(oracle_config_path);
                println!("Default oracle_config.yaml file is generated.");
                println!("Please, set the required parameters (node credentials, oracle_address)");
            } else {
                println!("oracle_config.yaml file already exists. Please, remove it and run again");
            }
        }
        Command::Bootstrap {
            yaml_config_name,
            generate_config_template,
        } => {
            if let Err(e) = (|| -> Result<(), anyhow::Error> {
                if generate_config_template {
                    cli_commands::bootstrap::generate_bootstrap_config_template(yaml_config_name)?;
                } else {
                    cli_commands::bootstrap::bootstrap(yaml_config_name)?;
                }
                Ok(())
            })() {
                {
                    error!("Fatal advanced-bootstrap error: {:?}", e);
                    std::process::exit(exitcode::SOFTWARE);
                }
            };
        }
        Command::PrintContractHashes => {
            print_contract_hashes();
        }
        oracle_command => handle_pool_command(oracle_command, &mut tokio_runtime, datapoint_source),
    }
}

/// Handle all non-bootstrap commands
fn handle_pool_command(
    command: Command,
    tokio_runtime: &mut tokio::runtime::Runtime,
    datapoint_source: RuntimeDataPointSource,
) {
    let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
    let height = BlockHeight(node_api.node.current_block_height().unwrap() as u32);
    log_on_launch();
    assert_wallet_unlocked(&node_api.node);
    register_and_save_scans(&node_api).unwrap();
    let op = OraclePool::new().unwrap();
    match command {
        Command::Run {
            read_only,
            enable_rest_api,
        } => {
            let (_, repost_receiver) = bounded::<bool>(1);

            // Start Oracle Core GET API Server
            if enable_rest_api {
                tokio_runtime.spawn(start_rest_server(repost_receiver));
            }
            loop {
                if let Err(e) = main_loop_iteration(&op, read_only, &datapoint_source) {
                    error!("error: {:?}", e);
                }
                // Delay loop restart
                thread::sleep(Duration::new(30, 0));
            }
        }

        Command::ExtractRewardTokens { rewards_address } => {
            if let Err(e) = cli_commands::extract_reward_tokens::extract_reward_tokens(
                // TODO: pass the NodeApi instance instead of these three
                &node_api,
                &node_api.node,
                &node_api.node,
                op.get_local_datapoint_box_source(),
                rewards_address,
                height,
            ) {
                error!("Fatal extract-rewards-token error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }

        Command::PrintRewardTokens => {
            if let Err(e) = cli_commands::print_reward_tokens::print_reward_tokens(
                op.get_local_datapoint_box_source(),
            ) {
                error!("Fatal print-rewards-token error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }

        Command::TransferOracleToken {
            oracle_token_address,
        } => {
            if let Err(e) = cli_commands::transfer_oracle_token::transfer_oracle_token(
                &node_api,
                &node_api.node,
                &node_api.node,
                op.get_local_datapoint_box_source(),
                oracle_token_address,
                height,
            ) {
                error!("Fatal transfer-oracle-token error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }

        Command::VoteUpdatePool {
            new_pool_box_address_hash_str,
            reward_token_id_str,
            reward_token_amount,
            update_box_creation_height,
        } => {
            if let Err(e) = cli_commands::vote_update_pool::vote_update_pool(
                &node_api,
                &node_api.node,
                &node_api.node,
                op.get_local_ballot_box_source(),
                new_pool_box_address_hash_str,
                reward_token_id_str,
                reward_token_amount,
                BlockHeight(update_box_creation_height),
                height,
            ) {
                error!("Fatal vote-update-pool error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }
        Command::UpdatePool {
            new_pool_box_hash,
            reward_token_id,
            reward_token_amount,
        } => {
            let new_reward_tokens =
                reward_token_id
                    .zip(reward_token_amount)
                    .map(|(token_id, amount)| Token {
                        token_id: TokenId::from(Digest32::try_from(token_id).unwrap()),
                        amount: amount.try_into().unwrap(),
                    });
            if let Err(e) = cli_commands::update_pool::update_pool(
                &op,
                &node_api,
                &node_api.node,
                &node_api.node,
                new_pool_box_hash,
                new_reward_tokens,
                height,
            ) {
                error!("Fatal update-pool error: {}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }
        Command::PrepareUpdate { update_file } => {
            if let Err(e) =
                cli_commands::prepare_update::prepare_update(update_file, &node_api, height)
            {
                error!("Fatal update error : {}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }
        Command::ImportPoolUpdate { pool_config_file } => {
            if let Err(e) = cli_commands::import_pool_update::import_pool_update(
                pool_config_file,
                &POOL_CONFIG.token_ids.oracle_token_id,
                POOL_CONFIG_FILE_PATH.get().unwrap(),
                op.get_local_datapoint_box_source(),
                &get_scans_file_path(),
            ) {
                error!("Fatal import pool update error : {}", e);
                std::process::exit(exitcode::SOFTWARE);
            } else {
                log::info!("pool config update imported successfully. Please, restart the oracle");
                std::process::exit(exitcode::OK);
            }
        }
        Command::Bootstrap { .. }
        | Command::PrintContractHashes
        | Command::GenerateOracleConfig => unreachable!(),
    }
}

fn main_loop_iteration(
    op: &OraclePool,
    read_only: bool,
    datapoint_source: &RuntimeDataPointSource,
) -> std::result::Result<(), anyhow::Error> {
    let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
    let height = BlockHeight(
        node_api
            .node
            .current_block_height()
            .context("Failed to get the current height")? as u32,
    );
    let network_change_address = node_api.get_change_address()?;
    let pool_state = match op.get_live_epoch_state() {
        Ok(live_epoch_state) => PoolState::LiveEpoch(live_epoch_state),
        Err(error) => {
            log::debug!("error getting live epoch state: {}", error);
            PoolState::NeedsBootstrap
        }
    };
    let epoch_length = POOL_CONFIG
        .refresh_box_wrapper_inputs
        .contract_inputs
        .contract_parameters()
        .epoch_length();
    if let Some(cmd) = process(pool_state, epoch_length, height) {
        log::debug!("Height {height}. Building action for command: {:?}", cmd);
        let build_action_res = build_action(
            cmd,
            op,
            &node_api,
            height,
            network_change_address.address(),
            datapoint_source,
        );
        if let Some(action) =
            log_and_continue_if_non_fatal(network_change_address.network(), build_action_res)?
        {
            if !read_only {
                execute_action(action, &node_api)?;
            }
        };
    }
    Ok(())
}

fn log_and_continue_if_non_fatal(
    network_prefix: NetworkPrefix,
    res: Result<PoolAction, PoolCommandError>,
) -> Result<Option<PoolAction>, PoolCommandError> {
    match res {
        Ok(action) => Ok(Some(action)),
        Err(PoolCommandError::RefreshActionError(RefreshActionError::FailedToReachConsensus {
            expected,
            found_public_keys,
            found_num,
        })) => {
            let found_oracle_addresses: String = found_public_keys
                .into_iter()
                .map(|pk| NetworkAddress::new(network_prefix, &Address::P2Pk(pk)).to_base58())
                .collect::<Vec<String>>()
                .join(", ");
            log::error!("Refresh failed, not enough datapoints. The minimum number of datapoints within the deviation range: required minumum {expected}, found {found_num} from addresses {found_oracle_addresses},");
            Ok(None)
        }
        Err(PoolCommandError::PublishDatapointActionError(
            PublishDatapointActionError::DataPointSource(e),
        )) => {
            log::error!("Failed to get datapoint with error: {}", e);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

fn log_on_launch() {
    log::info!("{}", APP_VERSION);
    if let Ok(config) = ORACLE_CONFIG_OPT.clone() {
        // log::info!("Token ids: {:?}", config.token_ids);
        log::info!("Oracle address: {}", config.oracle_address.to_base58());
    }
}
