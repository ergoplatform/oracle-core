use std::convert::TryFrom;

use crate::{
    box_kind::{
        BallotBoxWrapperInputs, OracleBoxWrapperInputs, PoolBoxWrapperInputs,
        RefreshBoxWrapperInputs, UpdateBoxWrapperInputs,
    },
    cli_commands::bootstrap::BootstrapConfig,
    contracts::{
        ballot::BallotContractError, oracle::OracleContractError, pool::PoolContractError,
        refresh::RefreshContractError, update::UpdateContractError,
    },
    datapoint_source::{DataPointSource, ExternalScript, PredefinedDataPointSource},
};
use anyhow::anyhow;
use derive_more::From;
use ergo_lib::{
    ergo_chain_types::Digest32,
    ergotree_ir::chain::address::NetworkAddress,
    ergotree_ir::chain::{ergo_box::box_value::BoxValue, token::TokenId},
    wallet::tx_builder::SUGGESTED_TX_FEE,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub oracle_address: NetworkAddress,
    pub data_point_source: Option<PredefinedDataPointSource>,
    pub data_point_source_custom_script: Option<String>,
    pub oracle_box_wrapper_inputs: OracleBoxWrapperInputs,
    pub pool_box_wrapper_inputs: PoolBoxWrapperInputs,
    pub refresh_box_wrapper_inputs: RefreshBoxWrapperInputs,
    pub update_box_wrapper_inputs: UpdateBoxWrapperInputs,
    pub ballot_box_wrapper_inputs: BallotBoxWrapperInputs,
    pub token_ids: TokenIds,
    pub rescan_height: u32,
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
    pub fn create(
        bootstrap: BootstrapConfig,
        token_ids: TokenIds,
        rescan_height: u32,
    ) -> Result<Self, OracleConfigError> {
        let oracle_box_wrapper_inputs = OracleBoxWrapperInputs::build_with(
            bootstrap.oracle_contract_parameters.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.oracle_token_id.clone(),
            token_ids.reward_token_id.clone(),
        )?;
        let refresh_box_wrapper_inputs = RefreshBoxWrapperInputs::build_with(
            bootstrap.refresh_contract_parameters.clone(),
            token_ids.refresh_nft_token_id.clone(),
            token_ids.oracle_token_id.clone(),
            token_ids.reward_token_id.clone(),
        )?;
        let pool_box_wrapper_inputs = PoolBoxWrapperInputs::build_with(
            bootstrap.pool_contract_parameters.clone(),
            token_ids.refresh_nft_token_id.clone(),
            token_ids.update_nft_token_id.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.reward_token_id.clone(),
        )?;
        let update_box_wrapper_inputs = UpdateBoxWrapperInputs::build_with(
            bootstrap.update_contract_parameters.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.ballot_token_id.clone(),
            token_ids.update_nft_token_id.clone(),
        )?;
        let ballot_box_wrapper_inputs = BallotBoxWrapperInputs::build_with(
            bootstrap.ballot_contract_parameters.clone(),
            token_ids.ballot_token_id.clone(),
            token_ids.update_nft_token_id.clone(),
        )?;
        Ok(OracleConfig {
            node_ip: bootstrap.node_ip,
            node_port: bootstrap.node_port,
            node_api_key: bootstrap.node_api_key,
            base_fee: bootstrap.base_fee,
            log_level: None,
            core_api_port: bootstrap.core_api_port,
            oracle_address: bootstrap.oracle_address,
            data_point_source: bootstrap.data_point_source,
            data_point_source_custom_script: bootstrap.data_point_source_custom_script,
            oracle_box_wrapper_inputs,
            pool_box_wrapper_inputs,
            refresh_box_wrapper_inputs,
            ballot_box_wrapper_inputs,
            update_box_wrapper_inputs,
            token_ids,
            rescan_height,
        })
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

#[derive(Debug, From, Error)]
pub enum OracleConfigError {
    #[error("Oracle contract error: {0}")]
    OracleContractError(OracleContractError),
    #[error("Refresh contract error: {0}")]
    RefreshContractError(RefreshContractError),
    #[error("Pool contract error: {0}")]
    PoolContractError(PoolContractError),
    #[error("Update contract error: {0}")]
    UpdateContractErro(UpdateContractError),
    #[error("Ballot contract error: {0}")]
    BallotContractErro(BallotContractError),
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

    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn token_ids_roundtrip() {
        let token_ids = generate_token_ids();
        let s = serde_yaml::to_string(&token_ids).unwrap();
        assert_eq!(token_ids, serde_yaml::from_str::<TokenIds>(&s).unwrap());
    }
}
