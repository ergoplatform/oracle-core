//! Types to allow oracle configuration to convert to and from Serde.

use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::address::{
    AddressEncoder, AddressEncoderError, NetworkAddress, NetworkPrefix,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

use crate::{
    contracts::{
        ballot::BallotContractParameters, oracle::OracleContractParameters,
        pool::PoolContractParameters, refresh::RefreshContractParameters,
    },
    datapoint_source::PredefinedDataPointSource,
    oracle_config::{
        BallotBoxWrapperParameters, CastBallotBoxVoteParameters, OracleConfig, TokenIds,
    },
};

/// Used to (de)serialize `OracleConfig` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OracleConfigSerde {
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
    oracle_contract_parameters: OracleContractParametersSerde,
    pool_contract_parameters: PoolContractParametersSerde,
    refresh_contract_parameters: RefreshContractParametersSerde,
    ballot_parameters: BallotBoxWrapperParametersSerde,
    token_ids: TokenIds,
}

impl TryFrom<OracleConfigSerde> for OracleConfig {
    type Error = AddressEncoderError;
    fn try_from(c: OracleConfigSerde) -> Result<Self, Self::Error> {
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

impl From<OracleConfig> for OracleConfigSerde {
    fn from(c: OracleConfig) -> Self {
        let oracle_contract_parameters = OracleContractParametersSerde {
            p2s: c.oracle_contract_parameters.p2s.to_base58(),
            pool_nft_index: c.oracle_contract_parameters.pool_nft_index,
        };
        let pool_contract_parameters = PoolContractParametersSerde {
            p2s: c.pool_contract_parameters.p2s.to_base58(),
            refresh_nft_index: c.pool_contract_parameters.refresh_nft_index,
            update_nft_index: c.pool_contract_parameters.update_nft_index,
        };
        let refresh_contract_parameters = RefreshContractParametersSerde {
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
        let ballot_parameters = BallotBoxWrapperParametersSerde {
            contract_parameters: BallotContractParametersSerde {
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
        OracleConfigSerde {
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
struct OracleContractParametersSerde {
    p2s: String,
    pool_nft_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoolContractParametersSerde {
    p2s: String,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshContractParametersSerde {
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
struct BallotContractParametersSerde {
    p2s: String,
    min_storage_rent_index: usize,
    min_storage_rent: u64,
    update_nft_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BallotBoxWrapperParametersSerde {
    contract_parameters: BallotContractParametersSerde,
    vote_parameters: Option<CastBallotBoxVoteParameters>,
    ballot_token_owner_address: String,
}
