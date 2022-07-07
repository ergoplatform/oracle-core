use std::convert::TryFrom;

use crate::{
    datapoint_source::{DataPointSource, ExternalScript, PredefinedDataPointSource},
    BlockDuration,
};
use anyhow::anyhow;
use ergo_lib::ergotree_ir::chain::{
    address::{NetworkAddress, NetworkPrefix},
    token::TokenId,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    pub node_ip: String,
    pub node_port: u16,
    pub node_api_key: String,
    pub refresh_nft: TokenId,
    pub update_nft: TokenId,
    pub reward_token_id: TokenId,
    pub ballot_token_id: TokenId,
    pub epoch_length: BlockDuration,
    pub buffer_length: BlockDuration,
    pub max_deviation_percent: u64,
    pub min_data_points: u64,
    pub ballot_box_min_storage_rent: u64,
    pub base_fee: u64,
    pub log_level: Option<LevelFilter>,
    pub oracle_pool_participant_token_id: TokenId,
    pub core_api_port: u16,
    pub oracle_address: String,
    /// Operator may not have a ballot token yet, but we assume that the address that 'owns' it is
    /// set here.
    pub ballot_token_owner_address: String,
    pub on_mainnet: bool,
    pub data_point_source: Option<PredefinedDataPointSource>,
    pub data_point_source_custom_script: Option<String>,
    pub oracle_contract_parameters: OracleContractParameters,
}

impl OracleConfig {
    fn load() -> Result<Self, anyhow::Error> {
        Self::load_from_str(&std::fs::read_to_string(DEFAULT_CONFIG_FILE_NAME)?)
    }

    fn load_from_str(config_str: &str) -> Result<OracleConfig, anyhow::Error> {
        serde_yaml::from_str(config_str).map_err(|e| anyhow!(e))
    }

    pub fn data_point_source(&self) -> Result<Box<dyn DataPointSource>, anyhow::Error> {
        let data_point_source: Box<dyn DataPointSource> = if let Some(external_script_name) =
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    try_from = "OracleContractParametersYaml",
    into = "OracleContractParametersYaml"
)]
pub struct OracleContractParameters {
    pub p2s: NetworkAddress,
    pub pool_nft_index: usize,
    pub pool_nft_token_id: TokenId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleContractParametersYaml {
    p2s: String,
    on_mainnet: bool,
    pool_nft_index: usize,
    pool_nft_token_id: TokenId,
}

impl TryFrom<OracleContractParametersYaml> for OracleContractParameters {
    type Error = String;

    fn try_from(p: OracleContractParametersYaml) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl Into<OracleContractParametersYaml> for OracleContractParameters {
    fn into(self) -> OracleContractParametersYaml {
        OracleContractParametersYaml {
            p2s: self.p2s.to_base58(),
            on_mainnet: self.p2s.network() == NetworkPrefix::Mainnet,
            pool_nft_index: self.pool_nft_index,
            pool_nft_token_id: self.pool_nft_token_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(pool_params.epoch_length, 20);
        assert_eq!(pool_params.buffer_length, 4);
        assert_eq!(pool_params.max_deviation_percent, 5);
        assert_eq!(pool_params.base_fee, 1000000);
    }
}
