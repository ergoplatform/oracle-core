use crate::{
    datapoint_source::{DataPointSource, ExternalScript, NanoAdaUsd, NanoErgUsd},
    BlockDuration,
};
use anyhow::anyhow;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use log::LevelFilter;
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

/// Node Parameters as defined in the `oracle-config.yaml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeParameters {
    pub node_ip: String,
    pub node_port: u16,
    pub node_api_key: String,
}

/// Pool Parameters as defined in the `oracle-config.yaml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolParameters {
    pub oracle_pool_nft: TokenId,
    pub refresh_nft: TokenId,
    pub reward_token_id: TokenId,
    pub epoch_length: BlockDuration,
    pub buffer_length: BlockDuration,
    pub max_deviation_percent: u64,
    pub min_data_points: u64,
    pub base_fee: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    pub node: NodeParameters,
    pub pool: PoolParameters,
    pub log_level: Option<LevelFilter>,
    pub oracle_pool_participant_token_id: TokenId,
    pub core_api_port: u16,
    pub oracle_address: String,
    pub on_mainnet: bool,
    pub data_point_source: String,
    pub data_point_source_custom_script: Option<String>,
}

impl OracleConfig {
    fn load() -> Result<Self, anyhow::Error> {
        Self::load_from_str(
            &std::fs::read_to_string(DEFAULT_CONFIG_FILE_NAME)
                .expect("Failed to open oracle-config.yaml"),
        )
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
            match &*self.data_point_source {
                "NanoErgUsd" => Box::new(NanoErgUsd),
                "NanoAdaUsd" => Box::new(NanoAdaUsd),
                _ => return Err(anyhow!("Config: data_point_source is invalid (must be one of 'NanoErgUsd' or 'NanoAdaUsd'")),
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
    ORACLE_CONFIG.node.node_ip.clone()
}

pub fn get_node_port() -> String {
    ORACLE_CONFIG.node.node_port.to_string()
}

/// Returns the `node_api_key`
pub fn get_node_api_key() -> String {
    ORACLE_CONFIG.node.node_api_key.clone()
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
        let pool_params = config.pool;
        assert_eq!(pool_params.epoch_length, 20);
        assert_eq!(pool_params.buffer_length, 4);
        assert_eq!(pool_params.max_deviation_percent, 5);
        assert_eq!(pool_params.base_fee, 1000000);
    }
}
