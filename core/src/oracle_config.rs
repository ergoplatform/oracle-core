use std::convert::TryFrom;

use crate::{
    cli_commands::bootstrap::{Addresses, BootstrapConfig},
    contracts::{
        ballot::BallotContractParameters, oracle::OracleContractParameters,
        pool::PoolContractParameters, refresh::RefreshContractParameters,
        update::UpdateContractParameters,
    },
    datapoint_source::{DataPointSource, ExternalScript, PredefinedDataPointSource},
};
use anyhow::anyhow;
use ergo_lib::{
    ergo_chain_types::Digest32,
    ergotree_ir::chain::{address::NetworkAddress, ergo_box::box_value::BoxValue, token::TokenId},
    wallet::tx_builder::SUGGESTED_TX_FEE,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

#[derive(Debug, Clone, Deserialize)]
#[serde(
    try_from = "crate::serde::OracleConfigSerde",
    into = "crate::serde::OracleConfigSerde"
)]
pub struct OracleConfig {
    pub node_ip: String,
    pub node_port: u16,
    pub node_api_key: String,
    pub base_fee: u64,
    pub log_level: Option<LevelFilter>,
    pub core_api_port: u16,
    pub oracle_address: String,
    pub data_point_source: Option<PredefinedDataPointSource>,
    pub data_point_source_custom_script: Option<String>,
    pub oracle_contract_parameters: OracleContractParameters,
    pub pool_contract_parameters: PoolContractParameters,
    pub refresh_contract_parameters: RefreshContractParameters,
    pub update_contract_parameters: UpdateContractParameters,
    pub ballot_parameters: BallotBoxWrapperParameters,
    pub token_ids: TokenIds,
    pub addresses: Addresses,
}

#[derive(Debug, Clone)]
pub struct BallotBoxWrapperParameters {
    pub contract_parameters: BallotContractParameters,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CastBallotBoxVoteParameters {
    pub pool_box_address_hash: Digest32,
    pub reward_token_id: TokenId,
    pub reward_token_quantity: u64,
    pub update_box_creation_height: i32,
}

/// Holds the token ids of every important token used by the oracle pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenIds {
    #[serde(
        serialize_with = "crate::serde::token_id_as_base64_string",
        deserialize_with = "crate::serde::token_id_from_base64"
    )]
    pub pool_nft_token_id: TokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base64_string",
        deserialize_with = "crate::serde::token_id_from_base64"
    )]
    pub refresh_nft_token_id: TokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base64_string",
        deserialize_with = "crate::serde::token_id_from_base64"
    )]
    pub update_nft_token_id: TokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base64_string",
        deserialize_with = "crate::serde::token_id_from_base64"
    )]
    pub oracle_token_id: TokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base64_string",
        deserialize_with = "crate::serde::token_id_from_base64"
    )]
    pub reward_token_id: TokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base64_string",
        deserialize_with = "crate::serde::token_id_from_base64"
    )]
    pub ballot_token_id: TokenId,
}

impl OracleConfig {
    pub fn create(bootstrap: BootstrapConfig, token_ids: TokenIds) -> Self {
        OracleConfig {
            node_ip: bootstrap.node_ip,
            node_port: bootstrap.node_port,
            node_api_key: bootstrap.node_api_key,
            // TODO: rename to tx_fee and use insteaf of BoxValue::SAFE_USER_MIN
            base_fee: todo!(), // bootstrap.base_fee,
            log_level: Some(LevelFilter::Info),
            // TODO: move to BootstrapConfig
            core_api_port: todo!(),
            oracle_address: todo!(),
            on_mainnet: todo!(),
            // TODO: move to BootstrapConfig
            data_point_source: Some(PredefinedDataPointSource::NanoErgUsd),
            // TODO: move to BootstrapConfig
            data_point_source_custom_script: todo!(),
            oracle_contract_parameters: bootstrap.oracle_contract_parameters,
            pool_contract_parameters: bootstrap.pool_contract_parameters,
            refresh_contract_parameters: bootstrap.refresh_contract_parameters,
            ballot_parameters: todo!(), // bootstrap.ballot_contract_parameters,
            update_contract_parameters: bootstrap.update_contract_parameters,
            token_ids,
            addresses: bootstrap.addresses,
        }
    }

    fn load() -> Result<Self, anyhow::Error> {
        Self::load_from_str(&std::fs::read_to_string(DEFAULT_CONFIG_FILE_NAME)?)
    }

    fn load_from_str(config_str: &str) -> Result<OracleConfig, anyhow::Error> {
        serde_yaml::from_str(config_str).map_err(|e| anyhow!(e))
    }

    pub fn data_point_source(
        &self,
    ) -> Result<Box<dyn DataPointSource + Send + Sync>, anyhow::Error> {
        let data_point_source: Box<dyn DataPointSource + Send + Sync> = if let Some(
            external_script_name,
        ) =
            self.data_point_source_custom_script.clone()
        {
            Box::new(ExternalScript::new(external_script_name.clone()))
        } else {
            match self.data_point_source {
                Some(datasource) => Box::new(datasource),
                _ => return Err(anyhow!("Config: data_point_source is invalid (must be one of 'NanoErgUsd', 'NanoErgXau' or 'NanoAdaUsd'")),
            }
        };
        Ok(data_point_source)
    }
}

lazy_static! {
    pub static ref ORACLE_CONFIG: OracleConfig = OracleConfig::load().unwrap();
    pub static ref MAYBE_ORACLE_CONFIG: Result<OracleConfig, String> =
        OracleConfig::load().map_err(|e| e.to_string());
    pub static ref BASE_FEE: BoxValue = MAYBE_ORACLE_CONFIG
        .as_ref()
        .map(|c| BoxValue::try_from(c.base_fee).unwrap())
        .unwrap_or_else(|_| SUGGESTED_TX_FEE());
}

/// Returns "core_api_port" from the config file
pub fn get_core_api_port() -> String {
    ORACLE_CONFIG.core_api_port.to_string()
}

pub fn get_node_ip() -> String {
    ORACLE_CONFIG.node_ip.clone()
}

pub fn get_node_port() -> String {
    ORACLE_CONFIG.node_port.to_string()
}

/// Returns the `node_api_key`
pub fn get_node_api_key() -> String {
    ORACLE_CONFIG.node_api_key.clone()
}

#[cfg(test)]
mod tests {
    use sigma_test_util::force_any_val;

    use super::*;

    #[ignore = "until config hierarchy and option names are finalized"]
    #[test]
    fn pool_parameter_parsing_works() {
        let yaml_string = "
            minimum_pool_box_value: 10000000
            epoch_length: 20
            buffer_length: 4
            max_deviation_percent: 5
            min_data_points: 4
            base_fee: 1000000
            ";
        let config = OracleConfig::load_from_str(yaml_string).unwrap();
        let pool_params = config;
        assert_eq!(pool_params.refresh_contract_parameters.epoch_length, 20);
        assert_eq!(pool_params.refresh_contract_parameters.buffer_length, 4);
        assert_eq!(
            pool_params
                .refresh_contract_parameters
                .max_deviation_percent,
            5
        );
        assert_eq!(pool_params.base_fee, 1000000);
    }

    #[test]
    fn token_ids_roundtrip() {
        let token_ids = TokenIds {
            pool_nft_token_id: force_any_val::<TokenId>(),
            refresh_nft_token_id: force_any_val::<TokenId>(),
            update_nft_token_id: force_any_val::<TokenId>(),
            oracle_token_id: force_any_val::<TokenId>(),
            reward_token_id: force_any_val::<TokenId>(),
            ballot_token_id: force_any_val::<TokenId>(),
        };

        let s = serde_yaml::to_string(&token_ids).unwrap();
        assert_eq!(token_ids, serde_yaml::from_str::<TokenIds>(&s).unwrap());
    }
}
