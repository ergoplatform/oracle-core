//! Types to allow oracle configuration to convert to and from Serde.

use std::convert::{TryFrom, TryInto};

use ergo_lib::ergotree_ir::chain::{
    address::{AddressEncoder, AddressEncoderError, NetworkAddress, NetworkPrefix},
    token::TokenId,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

use crate::{
    cli_commands::{
        bootstrap::{Addresses, BootstrapConfig, TokensToMint},
        prepare_update::{UpdateBootstrapConfig, UpdateTokensToMint},
    },
    contracts::{
        ballot::BallotContractParameters, oracle::OracleContractParameters,
        pool::PoolContractParameters, refresh::RefreshContractParameters,
        update::UpdateContractParameters,
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
    update_contract_parameters: UpdateContractParametersSerde,
    ballot_parameters: BallotBoxWrapperParametersSerde,
    token_ids: TokenIds,
    addresses: AddressesSerde,
}

impl TryFrom<OracleConfigSerde> for OracleConfig {
    type Error = AddressEncoderError;
    fn try_from(c: OracleConfigSerde) -> Result<Self, Self::Error> {
        let prefix = if c.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };

        let oracle_contract_parameters =
            OracleContractParameters::try_from(c.oracle_contract_parameters)?;

        let pool_contract_parameters =
            PoolContractParameters::try_from(c.pool_contract_parameters)?;

        let refresh_contract_parameters =
            RefreshContractParameters::try_from((c.refresh_contract_parameters, prefix))?;
        let update_contract_parameters =
            UpdateContractParameters::try_from((c.update_contract_parameters, prefix))?;

        let ballot_parameters = BallotBoxWrapperParameters {
            contract_parameters: BallotContractParameters::try_from(
                c.ballot_parameters.contract_parameters,
            )?,
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
            update_contract_parameters,
            ballot_parameters,
            token_ids: c.token_ids,
            addresses: Addresses::try_from((c.addresses, prefix))?,
        })
    }
}

impl From<OracleConfig> for OracleConfigSerde {
    fn from(c: OracleConfig) -> Self {
        let oracle_contract_parameters =
            OracleContractParametersSerde::from(c.oracle_contract_parameters);
        let pool_contract_parameters =
            PoolContractParametersSerde::from(c.pool_contract_parameters);
        let refresh_contract_parameters =
            RefreshContractParametersSerde::from(c.refresh_contract_parameters);
        let ballot_parameters = BallotBoxWrapperParametersSerde {
            contract_parameters: BallotContractParametersSerde::from(
                c.ballot_parameters.contract_parameters,
            ),
            vote_parameters: c.ballot_parameters.vote_parameters,
            ballot_token_owner_address: c.ballot_parameters.ballot_token_owner_address,
        };
        let update_contract_parameters =
            UpdateContractParametersSerde::from(c.update_contract_parameters);

        let prefix = if c.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
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
            update_contract_parameters,
            token_ids: c.token_ids,
            addresses: AddressesSerde::from((c.addresses, prefix)),
        }
    }
}

/// Used to (de)serialize `BootstrapConfig` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfigSerde {
    refresh_contract_parameters: RefreshContractParametersSerde,
    pool_contract_parameters: PoolContractParametersSerde,
    update_contract_parameters: UpdateContractParametersSerde,
    ballot_contract_parameters: BallotContractParametersSerde,
    tokens_to_mint: TokensToMint,
    node_ip: String,
    node_port: String,
    node_api_key: String,
    is_mainnet: bool,
    addresses: AddressesSerde,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AddressesSerde {
    address_for_oracle_tokens: String,
    wallet_address_for_chain_transaction: String,
}

impl From<(Addresses, NetworkPrefix)> for AddressesSerde {
    fn from(t: (Addresses, NetworkPrefix)) -> Self {
        let addresses = t.0;
        let prefix = t.1;
        let encoder = AddressEncoder::new(prefix);
        AddressesSerde {
            address_for_oracle_tokens: encoder.address_to_str(&addresses.address_for_oracle_tokens),
            wallet_address_for_chain_transaction: encoder
                .address_to_str(&addresses.wallet_address_for_chain_transaction),
        }
    }
}

impl TryFrom<(AddressesSerde, NetworkPrefix)> for Addresses {
    type Error = AddressEncoderError;
    fn try_from(t: (AddressesSerde, NetworkPrefix)) -> Result<Self, Self::Error> {
        let addresses = t.0;
        let prefix = t.1;
        let encoder = AddressEncoder::new(prefix);
        Ok(Addresses {
            address_for_oracle_tokens: encoder
                .parse_address_from_str(&addresses.address_for_oracle_tokens)?,
            wallet_address_for_chain_transaction: encoder
                .parse_address_from_str(&addresses.wallet_address_for_chain_transaction)?,
        })
    }
}

impl From<BootstrapConfig> for BootstrapConfigSerde {
    fn from(c: BootstrapConfig) -> Self {
        let prefix = if c.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        BootstrapConfigSerde {
            refresh_contract_parameters: RefreshContractParametersSerde::from(
                c.refresh_contract_parameters,
            ),
            pool_contract_parameters: PoolContractParametersSerde::from(c.pool_contract_parameters),
            update_contract_parameters: UpdateContractParametersSerde::from(
                c.update_contract_parameters,
            ),
            ballot_contract_parameters: BallotContractParametersSerde::from(
                c.ballot_contract_parameters,
            ),
            tokens_to_mint: c.tokens_to_mint,
            node_ip: c.node_ip,
            node_port: c.node_port,
            node_api_key: c.node_api_key,
            is_mainnet: c.on_mainnet,
            addresses: AddressesSerde::from((c.addresses, prefix)),
        }
    }
}

impl TryFrom<BootstrapConfigSerde> for BootstrapConfig {
    type Error = AddressEncoderError;

    fn try_from(c: BootstrapConfigSerde) -> Result<Self, Self::Error> {
        let prefix = if c.is_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        Ok(BootstrapConfig {
            refresh_contract_parameters: RefreshContractParameters::try_from((
                c.refresh_contract_parameters,
                prefix,
            ))?,
            pool_contract_parameters: PoolContractParameters::try_from(c.pool_contract_parameters)?,
            update_contract_parameters: UpdateContractParameters::try_from((
                c.update_contract_parameters,
                prefix,
            ))?,
            ballot_contract_parameters: BallotContractParameters::try_from(
                c.ballot_contract_parameters,
            )?,
            tokens_to_mint: c.tokens_to_mint,
            node_ip: c.node_ip,
            node_port: c.node_port,
            node_api_key: c.node_api_key,
            on_mainnet: c.is_mainnet,
            addresses: Addresses::try_from((c.addresses, prefix))?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleContractParametersSerde {
    p2s: String,
    pool_nft_index: usize,
}

impl From<OracleContractParameters> for OracleContractParametersSerde {
    fn from(p: OracleContractParameters) -> Self {
        OracleContractParametersSerde {
            p2s: AddressEncoder::new(NetworkPrefix::Mainnet).address_to_str(&p.p2s),
            pool_nft_index: p.pool_nft_index,
        }
    }
}

impl TryFrom<OracleContractParametersSerde> for OracleContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: OracleContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s =
            AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str(&contract.p2s)?;

        Ok(OracleContractParameters {
            p2s,
            pool_nft_index: contract.pool_nft_index,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoolContractParametersSerde {
    p2s: String,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

impl From<PoolContractParameters> for PoolContractParametersSerde {
    fn from(p: PoolContractParameters) -> Self {
        PoolContractParametersSerde {
            p2s: AddressEncoder::new(NetworkPrefix::Mainnet).address_to_str(&p.p2s),
            refresh_nft_index: p.refresh_nft_index,
            update_nft_index: p.update_nft_index,
        }
    }
}

impl TryFrom<PoolContractParametersSerde> for PoolContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: PoolContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s =
            AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str(&contract.p2s)?;
        Ok(PoolContractParameters {
            p2s,
            refresh_nft_index: contract.refresh_nft_index,
            update_nft_index: contract.update_nft_index,
        })
    }
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

impl From<RefreshContractParameters> for RefreshContractParametersSerde {
    fn from(p: RefreshContractParameters) -> Self {
        RefreshContractParametersSerde {
            p2s: p.p2s.to_base58(),
            pool_nft_index: p.pool_nft_index,
            oracle_token_id_index: p.oracle_token_id_index,
            min_data_points_index: p.min_data_points_index,
            min_data_points: p.min_data_points,
            buffer_index: p.buffer_index,
            buffer_length: p.buffer_length,
            max_deviation_percent_index: p.max_deviation_percent_index,
            max_deviation_percent: p.max_deviation_percent,
            epoch_length_index: p.epoch_length_index,
            epoch_length: p.epoch_length,
        }
    }
}

impl TryFrom<(RefreshContractParametersSerde, NetworkPrefix)> for RefreshContractParameters {
    type Error = AddressEncoderError;
    fn try_from(t: (RefreshContractParametersSerde, NetworkPrefix)) -> Result<Self, Self::Error> {
        let prefix = t.1;
        let contract = t.0;
        let refresh_contract_address =
            AddressEncoder::new(prefix).parse_address_from_str(&contract.p2s)?;
        Ok(RefreshContractParameters {
            p2s: NetworkAddress::new(prefix, &refresh_contract_address),
            pool_nft_index: contract.pool_nft_index,
            oracle_token_id_index: contract.oracle_token_id_index,
            min_data_points_index: contract.min_data_points_index,
            min_data_points: contract.min_data_points,
            buffer_index: contract.buffer_index,
            buffer_length: contract.buffer_length,
            max_deviation_percent_index: contract.max_deviation_percent_index,
            max_deviation_percent: contract.max_deviation_percent,
            epoch_length_index: contract.epoch_length_index,
            epoch_length: contract.epoch_length,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BallotContractParametersSerde {
    p2s: String,
    min_storage_rent_index: usize,
    min_storage_rent: u64,
    update_nft_index: usize,
}

impl From<BallotContractParameters> for BallotContractParametersSerde {
    fn from(c: BallotContractParameters) -> Self {
        BallotContractParametersSerde {
            p2s: AddressEncoder::new(NetworkPrefix::Mainnet).address_to_str(&c.p2s),
            min_storage_rent_index: c.min_storage_rent_index,
            min_storage_rent: c.min_storage_rent,
            update_nft_index: c.update_nft_index,
        }
    }
}

impl TryFrom<BallotContractParametersSerde> for BallotContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: BallotContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s =
            AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str(&contract.p2s)?;
        Ok(BallotContractParameters {
            p2s,
            min_storage_rent_index: contract.min_storage_rent_index,
            min_storage_rent: contract.min_storage_rent,
            update_nft_index: contract.update_nft_index,
        })
    }
}

/// Used to (de)serialize `OracleContractParameters` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateContractParametersSerde {
    p2s: String,
    pool_nft_index: usize,
    ballot_token_index: usize,
    min_votes_index: usize,
    min_votes: u64,
}

impl TryFrom<(UpdateContractParametersSerde, NetworkPrefix)> for UpdateContractParameters {
    type Error = AddressEncoderError;

    fn try_from(t: (UpdateContractParametersSerde, NetworkPrefix)) -> Result<Self, Self::Error> {
        let prefix = t.1;
        let contract = t.0;
        let address = AddressEncoder::new(prefix).parse_address_from_str(&contract.p2s)?;
        Ok(UpdateContractParameters {
            p2s: NetworkAddress::new(prefix, &address),
            pool_nft_index: contract.pool_nft_index,
            ballot_token_index: contract.ballot_token_index,
            min_votes_index: contract.min_votes_index,
            min_votes: contract.min_votes,
        })
    }
}

impl From<UpdateContractParameters> for UpdateContractParametersSerde {
    fn from(p: UpdateContractParameters) -> Self {
        UpdateContractParametersSerde {
            p2s: p.p2s.to_base58(),
            pool_nft_index: p.pool_nft_index,
            ballot_token_index: p.ballot_token_index,
            min_votes_index: p.min_votes_index,
            min_votes: p.min_votes,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct UpdateBootstrapConfigSerde {
    pool_contract_parameters: Option<PoolContractParametersSerde>,
    refresh_contract_parameters: Option<RefreshContractParametersSerde>,
    update_contract_parameters: Option<UpdateContractParametersSerde>,
    tokens_to_mint: UpdateTokensToMint,
    addresses: AddressesSerde,
}

impl TryFrom<UpdateBootstrapConfigSerde> for UpdateBootstrapConfig {
    type Error = AddressEncoderError;
    fn try_from(c: UpdateBootstrapConfigSerde) -> Result<UpdateBootstrapConfig, Self::Error> {
        let prefix = if crate::oracle_config::ORACLE_CONFIG.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        let pool_contract_parameters = c
            .pool_contract_parameters
            .map(|r| (r, prefix).try_into())
            .transpose()?;
        let refresh_contract_parameters = c
            .refresh_contract_parameters
            .map(|r| (r, prefix).try_into())
            .transpose()?;
        let update_contract_parameters = c
            .update_contract_parameters
            .map(|r| (r, prefix).try_into())
            .transpose()?;
        let addresses = (c.addresses, prefix).try_into()?;
        Ok(UpdateBootstrapConfig {
            pool_contract_parameters,
            refresh_contract_parameters,
            update_contract_parameters,
            tokens_to_mint: c.tokens_to_mint,
            addresses,
        })
    }
}

pub(crate) fn token_id_as_base64_string<S>(
    value: &TokenId,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let bytes: Vec<u8> = value.clone().into();
    serializer.serialize_str(&base64::encode(bytes))
}

pub(crate) fn token_id_from_base64<'de, D>(deserializer: D) -> Result<TokenId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    // Interesting fact: `s` can't be of type `&str` otherwise we get the following error at
    // runtime:
    //   "invalid type: string ..., expected a borrowed string"
    let s: String = serde::de::Deserialize::deserialize(deserializer)?;
    TokenId::from_base64(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BallotBoxWrapperParametersSerde {
    contract_parameters: BallotContractParametersSerde,
    vote_parameters: Option<CastBallotBoxVoteParameters>,
    ballot_token_owner_address: String,
}
