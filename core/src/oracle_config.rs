use std::convert::TryFrom;

use crate::{
    contracts::{
        ballot::BallotContractParameters, oracle::OracleContractParameters,
        pool::PoolContractParameters, refresh::RefreshContractParameters,
    },
    datapoint_source::{DataPointSource, ExternalScript, PredefinedDataPointSource},
};
use anyhow::anyhow;
use ergo_lib::ergotree_ir::chain::{
    address::{AddressEncoder, AddressEncoderError, NetworkAddress, NetworkPrefix},
    token::TokenId,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "OracleConfigYaml", into = "OracleConfigYaml")]
pub struct OracleConfig {
    pub node_ip: String,
    pub node_port: u16,
    pub node_api_key: String,
    pub base_fee: u64,
    pub log_level: Option<LevelFilter>,
    pub core_api_port: u16,
    pub oracle_address: String,
    pub on_mainnet: bool,
    pub data_point_source: Option<PredefinedDataPointSource>,
    pub data_point_source_custom_script: Option<String>,
    pub oracle_contract_parameters: OracleContractParameters,
    pub pool_contract_parameters: PoolContractParameters,
    pub refresh_contract_parameters: RefreshContractParameters,
    pub ballot_parameters: BallotBoxWrapperParameters,
    pub token_ids: TokenIds,
}

#[derive(Debug, Clone)]
pub struct BallotBoxWrapperParameters {
    pub contract_parameters: BallotContractParameters,
    pub vote_parameters: Option<CastBallotBoxVoteParameters>,
    /// Operator may not have a ballot token yet, but we assume that the address that 'owns' it is
    /// set here.
    pub ballot_token_owner_address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CastBallotBoxVoteParameters {
    pub pool_box_address_hash: String,
    pub reward_token_id: TokenId,
    pub reward_token_quantity: u32,
}

/// Holds the token ids of every important token used by the oracle pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIds {
    pub pool_nft_token_id: TokenId,
    pub refresh_nft_token_id: TokenId,
    pub update_nft_token_id: TokenId,
    pub oracle_token_id: TokenId,
    pub reward_token_id: TokenId,
    pub ballot_token_id: TokenId,
}

impl OracleConfig {
    fn load() -> Result<Self, anyhow::Error> {
        let config = Self::load_from_str(&std::fs::read_to_string(DEFAULT_CONFIG_FILE_NAME)?)?;

        // Check network prefixes
        let prefix = if config.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        if prefix == config.oracle_contract_parameters.p2s.network()
            && prefix == config.pool_contract_parameters.p2s.network()
            && prefix == config.refresh_contract_parameters.p2s.network()
            && prefix == config.ballot_parameters.contract_parameters.p2s.network()
        {
            Ok(config)
        } else {
            Err(anyhow!("Network prefixes are not constant"))
        }
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

/// Used to (de)serialize `OracleConfig` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleConfigYaml {
    node_ip: String,
    node_port: u16,
    node_api_key: String,
    base_fee: u64,
    log_level: Option<LevelFilter>,
    core_api_port: u16,
    oracle_address: String,
    on_mainnet: bool,
    data_point_source: Option<PredefinedDataPointSource>,
    data_point_source_custom_script: Option<String>,
    oracle_contract_parameters: OracleContractParametersYaml,
    pool_contract_parameters: PoolContractParametersYaml,
    refresh_contract_parameters: RefreshContractParametersYaml,
    ballot_parameters: BallotBoxWrapperParametersYaml,
    token_ids: TokenIds,
}

impl TryFrom<OracleConfigYaml> for OracleConfig {
    type Error = AddressEncoderError;
    fn try_from(c: OracleConfigYaml) -> Result<Self, Self::Error> {
        let prefix = if c.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };

        let oracle_contract_address = AddressEncoder::new(prefix)
            .parse_address_from_str(&c.oracle_contract_parameters.p2s)?;

        let oracle_contract_parameters = OracleContractParameters {
            p2s: NetworkAddress::new(prefix, &oracle_contract_address),
            pool_nft_index: c.oracle_contract_parameters.pool_nft_index,
        };

        let pool_contract_address =
            AddressEncoder::new(prefix).parse_address_from_str(&c.pool_contract_parameters.p2s)?;
        let pool_contract_parameters = PoolContractParameters {
            p2s: NetworkAddress::new(prefix, &pool_contract_address),
            refresh_nft_index: c.pool_contract_parameters.refresh_nft_index,
            update_nft_index: c.pool_contract_parameters.update_nft_index,
        };

        let refresh_contract_address = AddressEncoder::new(prefix)
            .parse_address_from_str(&c.refresh_contract_parameters.p2s)?;
        let refresh_contract_parameters = RefreshContractParameters {
            p2s: NetworkAddress::new(prefix, &refresh_contract_address),
            pool_nft_index: c.refresh_contract_parameters.pool_nft_index,
            oracle_token_id_index: c.refresh_contract_parameters.oracle_token_id_index,
            min_data_points_index: c.refresh_contract_parameters.min_data_points_index,
            min_data_points: c.refresh_contract_parameters.min_data_points,
            buffer_index: c.refresh_contract_parameters.buffer_index,
            buffer_length: c.refresh_contract_parameters.buffer_length,
            max_deviation_percent_index: c.refresh_contract_parameters.max_deviation_percent_index,
            max_deviation_percent: c.refresh_contract_parameters.max_deviation_percent,
            epoch_length_index: c.refresh_contract_parameters.epoch_length_index,
            epoch_length: c.refresh_contract_parameters.epoch_length,
        };

        let ballot_contract_address = AddressEncoder::new(prefix)
            .parse_address_from_str(&c.ballot_parameters.contract_parameters.p2s)?;
        let ballot_parameters = BallotBoxWrapperParameters {
            contract_parameters: BallotContractParameters {
                p2s: NetworkAddress::new(prefix, &ballot_contract_address),
                min_storage_rent_index: c
                    .ballot_parameters
                    .contract_parameters
                    .min_storage_rent_index,
                min_storage_rent: c.ballot_parameters.contract_parameters.min_storage_rent,
                update_nft_index: c.ballot_parameters.contract_parameters.update_nft_index,
            },
            vote_parameters: c.ballot_parameters.vote_parameters,
            ballot_token_owner_address: c.ballot_parameters.ballot_token_owner_address,
        };
        Ok(OracleConfig {
            node_ip: c.node_ip,
            node_port: c.node_port,
            node_api_key: c.node_api_key,
            base_fee: c.base_fee,
            log_level: c.log_level,
            core_api_port: c.core_api_port,
            oracle_address: c.oracle_address,
            on_mainnet: c.on_mainnet,
            data_point_source: c.data_point_source,
            data_point_source_custom_script: c.data_point_source_custom_script,
            oracle_contract_parameters,
            pool_contract_parameters,
            refresh_contract_parameters,
            ballot_parameters,
            token_ids: c.token_ids,
        })
    }
}

impl From<OracleConfig> for OracleConfigYaml {
    fn from(c: OracleConfig) -> Self {
        let oracle_contract_parameters = OracleContractParametersYaml {
            p2s: c.oracle_contract_parameters.p2s.to_base58(),
            pool_nft_index: c.oracle_contract_parameters.pool_nft_index,
        };
        let pool_contract_parameters = PoolContractParametersYaml {
            p2s: c.pool_contract_parameters.p2s.to_base58(),
            refresh_nft_index: c.pool_contract_parameters.refresh_nft_index,
            update_nft_index: c.pool_contract_parameters.update_nft_index,
        };
        let refresh_contract_parameters = RefreshContractParametersYaml {
            p2s: c.refresh_contract_parameters.p2s.to_base58(),
            pool_nft_index: c.refresh_contract_parameters.pool_nft_index,
            oracle_token_id_index: c.refresh_contract_parameters.oracle_token_id_index,
            min_data_points_index: c.refresh_contract_parameters.min_data_points_index,
            min_data_points: c.refresh_contract_parameters.min_data_points,
            buffer_index: c.refresh_contract_parameters.buffer_index,
            buffer_length: c.refresh_contract_parameters.buffer_length,
            max_deviation_percent_index: c.refresh_contract_parameters.max_deviation_percent_index,
            max_deviation_percent: c.refresh_contract_parameters.max_deviation_percent,
            epoch_length_index: c.refresh_contract_parameters.epoch_length_index,
            epoch_length: c.refresh_contract_parameters.epoch_length,
        };
        let ballot_parameters = BallotBoxWrapperParametersYaml {
            contract_parameters: BallotContractParametersYaml {
                p2s: c.ballot_parameters.contract_parameters.p2s.to_base58(),
                min_storage_rent_index: c
                    .ballot_parameters
                    .contract_parameters
                    .min_storage_rent_index,
                min_storage_rent: c.ballot_parameters.contract_parameters.min_storage_rent,
                update_nft_index: c.ballot_parameters.contract_parameters.update_nft_index,
            },
            vote_parameters: c.ballot_parameters.vote_parameters,
            ballot_token_owner_address: c.ballot_parameters.ballot_token_owner_address,
        };
        OracleConfigYaml {
            node_ip: c.node_ip,
            node_port: c.node_port,
            node_api_key: c.node_api_key,
            base_fee: c.base_fee,
            log_level: c.log_level,
            core_api_port: c.core_api_port,
            oracle_address: c.oracle_address,
            on_mainnet: c.on_mainnet,
            data_point_source: c.data_point_source,
            data_point_source_custom_script: c.data_point_source_custom_script,
            oracle_contract_parameters,
            pool_contract_parameters,
            refresh_contract_parameters,
            ballot_parameters,
            token_ids: c.token_ids,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleContractParametersYaml {
    p2s: String,
    pool_nft_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoolContractParametersYaml {
    p2s: String,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshContractParametersYaml {
    p2s: String,
    pool_nft_index: usize,
    oracle_token_id_index: usize,
    min_data_points_index: usize,
    min_data_points: u64,
    buffer_index: usize,
    buffer_length: u64,
    max_deviation_percent_index: usize,
    max_deviation_percent: u64,
    epoch_length_index: usize,
    epoch_length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BallotContractParametersYaml {
    p2s: String,
    min_storage_rent_index: usize,
    min_storage_rent: u64,
    update_nft_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BallotBoxWrapperParametersYaml {
    contract_parameters: BallotContractParametersYaml,
    vote_parameters: Option<CastBallotBoxVoteParameters>,
    ballot_token_owner_address: String,
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
}
