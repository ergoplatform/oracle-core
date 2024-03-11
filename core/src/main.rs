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
// #![allow(clippy::correctness)]
// #![allow(clippy::almost_swapped)]

#[macro_use]
extern crate lazy_static;

mod action_report;
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
mod metrics;
mod migrate;
mod monitor;
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
mod util;
mod wallet;

#[cfg(test)]
mod tests;

use action_report::ActionReportStorage;
use action_report::PoolActionReport;
use actions::PoolAction;
use anyhow::anyhow;
use anyhow::Context;
use clap::{Parser, Subcommand};
use crossbeam::channel::bounded;
use datapoint_source::RuntimeDataPointSource;
use ergo_lib::ergo_chain_types::Digest32;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenAmount;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use log::error;
use log::LevelFilter;
use metrics::start_metrics_server;
use metrics::update_metrics;
use node_interface::node_api::NodeApi;
use node_interface::try_ensure_wallet_unlocked;
use oracle_config::ORACLE_CONFIG;
use oracle_config::ORACLE_SECRETS;
use oracle_state::OraclePool;
use oracle_types::BlockHeight;
use pool_commands::build_action;
use pool_commands::publish_datapoint::PublishDatapointActionError;
use pool_commands::refresh::RefreshActionError;
use pool_commands::PoolCommandError;
use pool_config::DEFAULT_POOL_CONFIG_FILE_NAME;
use pool_config::POOL_CONFIG;
use scans::get_scans_file_path;
use scans::wait_for_node_rescan;
use spec_token::RewardTokenId;
use spec_token::SpecToken;
use spec_token::TokenIdKind;
use state::process;
use state::PoolState;
use std::convert::TryFrom;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use crate::actions::execute_action;
use crate::address_util::pks_to_network_addresses;
use crate::api::start_rest_server;
use crate::box_kind::BallotBox;
use crate::contracts::ballot::BallotContract;
use crate::default_parameters::print_contract_hashes;
use crate::migrate::check_migration_to_split_config;
use crate::oracle_config::OracleConfig;
use crate::oracle_config::DEFAULT_ORACLE_CONFIG_FILE_NAME;
use crate::oracle_config::ORACLE_CONFIG_FILE_PATH;
use crate::oracle_config::ORACLE_CONFIG_OPT;
use crate::pool_config::POOL_CONFIG_FILE_PATH;
use crate::scans::NodeScanRegistry;

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
        /// The creation height of the existing update box.
        update_box_creation_height: u32,
        /// The base16-encoded reward token id of the new pool box (if minted)
        reward_token_id_str: Option<String>,
        /// The reward token amount in the pool box at the time of update transaction is committed (if minted).
        reward_token_amount: Option<u64>,
    },
    /// Initiate the Update Pool transaction.
    /// Updated config file `pool_config_updated.yaml` is expected to be in the current directory
    /// and must be created using --prepare-update command first
    UpdatePool {
        /// New reward token id (only if minted)
        reward_token_id: Option<String>,
        /// New reward token amount (only if minted)
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
        .as_ref()
        .map(|c| c.log_level)
        .ok()
        .flatten();
    logging::setup_log(cmdline_log_level, config_log_level, &data_dir_path);

    scans::SCANS_DIR_PATH.set(data_dir_path).unwrap();

    let action_report_storage: Arc<RwLock<ActionReportStorage>> =
        Arc::new(RwLock::new(ActionReportStorage::new()));

    log_on_launch();
    let node_api = NodeApi::new(
        ORACLE_SECRETS.node_api_key.clone(),
        ORACLE_SECRETS.wallet_password.clone(),
        &ORACLE_CONFIG.node_url,
    );
    try_ensure_wallet_unlocked(&node_api);
    wait_for_node_rescan(&node_api).unwrap();

    let pool_config = &POOL_CONFIG;

    let change_address = node_api
        .get_change_address()
        .expect("failed to get change address from the node");
    let network_prefix = change_address.network();

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
        Command::Run {
            read_only,
            enable_rest_api,
        } => {
            let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
            let (_, repost_receiver) = bounded::<bool>(1);

            let node_scan_registry =
                NodeScanRegistry::ensure_node_registered_scans(&node_api, pool_config).unwrap();
            let oracle_pool = Arc::new(OraclePool::new(&node_scan_registry).unwrap());
            let datapoint_source = RuntimeDataPointSource::new(
                POOL_CONFIG.data_point_source,
                ORACLE_CONFIG.data_point_source_custom_script.clone(),
            )
            .unwrap();

            // Start Oracle Core GET API Server
            if enable_rest_api {
                let op_clone = oracle_pool.clone();
                tokio_runtime.spawn(async {
                    if let Err(e) =
                        start_rest_server(repost_receiver, op_clone, ORACLE_CONFIG.core_api_port)
                            .await
                    {
                        error!("An error occurred while starting the REST server: {}", e);
                        std::process::exit(exitcode::SOFTWARE);
                    }
                });
            }
            if let Some(metrics_port) = ORACLE_CONFIG.metrics_port {
                tokio_runtime.spawn(async move {
                    if let Err(e) = start_metrics_server(metrics_port).await {
                        error!("An error occurred while starting the metrics server: {}", e);
                        std::process::exit(exitcode::SOFTWARE);
                    }
                });
            }
            loop {
                if let Err(e) = main_loop_iteration(
                    oracle_pool.clone(),
                    read_only,
                    &datapoint_source,
                    &node_api,
                    action_report_storage.clone(),
                    &change_address,
                ) {
                    error!("error: {:?}", e);
                }
                // Delay loop restart
                thread::sleep(Duration::new(30, 0));
            }
        }
        oracle_command => handle_pool_command(oracle_command, &node_api, network_prefix),
    }
}

/// Handle all other commands
fn handle_pool_command(command: Command, node_api: &NodeApi, network_prefix: NetworkPrefix) {
    let height = BlockHeight(node_api.node.current_block_height().unwrap() as u32);
    let node_scan_registry = NodeScanRegistry::load().unwrap();
    let op = OraclePool::new(&node_scan_registry).unwrap();
    match command {
        Command::ExtractRewardTokens { rewards_address } => {
            if let Err(e) = cli_commands::extract_reward_tokens::extract_reward_tokens(
                // TODO: pass the NodeApi instance instead of these three
                node_api,
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
                node_api,
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
            let reward_token_opt = check_reward_token_opt(reward_token_id_str, reward_token_amount);
            log::debug!(
                "found ballot boxes: {:?}",
                op.get_ballot_boxes_source()
                    .get_ballot_boxes()
                    .unwrap()
                    .into_iter()
                    .map(|b| (
                        b.get_box().box_id(),
                        b.ballot_token_owner_address(network_prefix).to_base58()
                    ))
                    .collect::<Vec<_>>()
            );
            let ballot_contract = BallotContract::checked_load(
                &POOL_CONFIG.ballot_box_wrapper_inputs.contract_inputs,
            )
            .unwrap();
            if let Err(e) = cli_commands::vote_update_pool::vote_update_pool(
                node_api,
                &node_api.node,
                &node_api.node,
                op.get_local_ballot_box_source(),
                new_pool_box_address_hash_str,
                reward_token_opt,
                BlockHeight(update_box_creation_height),
                height,
                &ballot_contract,
            ) {
                error!("Fatal vote-update-pool error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }
        Command::UpdatePool {
            reward_token_id,
            reward_token_amount,
        } => {
            let reward_token_opt = check_reward_token_opt(reward_token_id, reward_token_amount);
            if let Err(e) = cli_commands::update_pool::update_pool(
                &op,
                node_api,
                &node_api.node,
                &node_api.node,
                reward_token_opt,
                height,
            ) {
                error!("Fatal update-pool error: {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }
        Command::PrepareUpdate { update_file } => {
            if let Err(e) =
                cli_commands::prepare_update::prepare_update(update_file, node_api, height)
            {
                error!("Fatal update error : {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
        }
        Command::ImportPoolUpdate { pool_config_file } => {
            if op.get_pool_box_source().get_pool_box().is_ok() {
                error!("Seems like update-pool command is missing (pool box is found).");
                std::process::exit(exitcode::SOFTWARE);
            }
            if let Err(e) = cli_commands::import_pool_update::import_pool_update(
                pool_config_file,
                &POOL_CONFIG.token_ids.oracle_token_id,
                &POOL_CONFIG.token_ids.reward_token_id,
                POOL_CONFIG_FILE_PATH.get().unwrap(),
                op.get_local_datapoint_box_source(),
                &get_scans_file_path(),
                node_scan_registry,
                node_api,
            ) {
                error!("Fatal import pool update error : {:?}", e);
                std::process::exit(exitcode::SOFTWARE);
            } else {
                log::info!("pool config update imported successfully. Please, restart the oracle");
                std::process::exit(exitcode::OK);
            }
        }
        Command::Bootstrap { .. }
        | Command::PrintContractHashes
        | Command::GenerateOracleConfig
        | Command::Run { .. } => unreachable!(),
    }
}

fn main_loop_iteration(
    oracle_pool: Arc<OraclePool>,
    read_only: bool,
    datapoint_source: &RuntimeDataPointSource,
    node_api: &NodeApi,
    report_storage: Arc<RwLock<ActionReportStorage>>,
    change_address: &NetworkAddress,
) -> std::result::Result<(), anyhow::Error> {
    if !node_api.node.wallet_status()?.unlocked {
        return Err(anyhow!("Wallet is locked!"));
    }
    let height = BlockHeight(
        node_api
            .node
            .current_block_height()
            .context("Failed to get the current height")? as u32,
    );
    let pool_state = match oracle_pool.get_live_epoch_state() {
        Ok(live_epoch_state) => PoolState::LiveEpoch(live_epoch_state),
        Err(error) => {
            log::error!("error getting live epoch state: {:?}", error);
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
        let build_action_tuple_res = build_action(
            cmd,
            &oracle_pool,
            node_api,
            height,
            change_address.address(),
            datapoint_source,
        );
        if let Some((action, report)) =
            log_and_continue_if_non_fatal(change_address.network(), build_action_tuple_res)?
        {
            if !read_only {
                execute_action(action, node_api)?;
                report_storage.write().unwrap().add(report);
            }
        };
    }
    update_metrics(oracle_pool)?;
    Ok(())
}

fn log_and_continue_if_non_fatal(
    network_prefix: NetworkPrefix,
    res: Result<(PoolAction, PoolActionReport), PoolCommandError>,
) -> Result<Option<(PoolAction, PoolActionReport)>, anyhow::Error> {
    match res {
        Ok(tuple) => Ok(Some(tuple)),
        Err(PoolCommandError::RefreshActionError(RefreshActionError::FailedToReachConsensus {
            expected,
            found_public_keys,
            found_num,
        })) => {
            let found_oracle_addresses: String =
                pks_to_network_addresses(found_public_keys, network_prefix)
                    .into_iter()
                    .map(|net_addr| net_addr.to_base58())
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
        Err(e) => Err(e.into()),
    }
}

fn log_on_launch() {
    log::info!("{}", APP_VERSION);
    let oracle_address_opt = ORACLE_CONFIG_OPT.as_ref().map(|c| c.oracle_address.clone());
    if let Ok(oracle_address) = oracle_address_opt {
        // log::info!("Token ids: {:?}", config.token_ids);
        log::info!("Oracle address: {}", oracle_address.to_base58());
    }
}

fn check_reward_token_opt(
    reward_token_id_str: Option<String>,
    reward_token_amount: Option<u64>,
) -> Option<SpecToken<RewardTokenId>> {
    match (reward_token_id_str, reward_token_amount) {
        (None, None) => None,
        (None, Some(_)) => {
            panic!("reward_token_amount is set, but reward_token_id is not set")
        }
        (Some(_), None) => {
            panic!("reward_token_id is set, but reward_token_amount is not set")
        }
        (Some(reward_token_id_str), Some(reward_token_amount)) => Some({
            let reward_token_id: TokenId = Digest32::try_from(reward_token_id_str).unwrap().into();
            SpecToken {
                token_id: RewardTokenId::from_token_id_unchecked(reward_token_id),
                amount: TokenAmount::try_from(reward_token_amount).unwrap(),
            }
        }),
    }
}
