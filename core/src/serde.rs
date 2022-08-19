//! Types to allow oracle configuration to convert to and from Serde.

use std::convert::{TryFrom, TryInto};

use derive_more::From;
use ergo_lib::ergotree_ir::chain::{
    address::{AddressEncoder, AddressEncoderError, NetworkPrefix},
    token::TokenId,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    oracle_config::{OracleConfig, TokenIds},
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
    data_point_source: Option<PredefinedDataPointSource>,
    data_point_source_custom_script: Option<String>,
    oracle_contract_parameters: OracleContractParametersSerde,
    pool_contract_parameters: PoolContractParametersSerde,
    refresh_contract_parameters: RefreshContractParametersSerde,
    update_contract_parameters: UpdateContractParametersSerde,
    ballot_contract_parameters: BallotContractParametersSerde,
    token_ids: TokenIds,
    addresses: AddressesSerde,
}

#[derive(Debug, Error, From)]
pub enum SerdeConversionError {
    #[error("Serde conversion error: AddressEncoder {0}")]
    AddressEncoder(AddressEncoderError),
    #[error("Serde conversion error: Network prefixes of addresses differ")]
    NetworkPrefixesDiffer,
}

impl From<OracleConfig> for OracleConfigSerde {
    fn from(c: OracleConfig) -> Self {
        let oracle_contract_parameters =
            OracleContractParametersSerde::from(c.oracle_contract_parameters);
        let pool_contract_parameters =
            PoolContractParametersSerde::from(c.pool_contract_parameters);
        let refresh_contract_parameters =
            RefreshContractParametersSerde::from(c.refresh_contract_parameters);
        let ballot_contract_parameters =
            BallotContractParametersSerde::from(c.ballot_contract_parameters);
        let update_contract_parameters =
            UpdateContractParametersSerde::from(c.update_contract_parameters);

        OracleConfigSerde {
            node_ip: c.node_ip,
            node_port: c.node_port,
            node_api_key: c.node_api_key,
            base_fee: c.base_fee,
            log_level: c.log_level,
            core_api_port: c.core_api_port,
            oracle_address: c.oracle_address.to_base58(),
            data_point_source: c.data_point_source,
            data_point_source_custom_script: c.data_point_source_custom_script,
            oracle_contract_parameters,
            pool_contract_parameters,
            refresh_contract_parameters,
            ballot_contract_parameters,
            update_contract_parameters,
            token_ids: c.token_ids,
            addresses: AddressesSerde::from(c.addresses),
        }
    }
}

impl TryFrom<OracleConfigSerde> for OracleConfig {
    type Error = SerdeConversionError;
    fn try_from(c: OracleConfigSerde) -> Result<Self, Self::Error> {
        let oracle_contract_parameters =
            OracleContractParameters::try_from(c.oracle_contract_parameters)?;
        let oracle_contract_prefix = oracle_contract_parameters.p2s.network();

        let pool_contract_parameters =
            PoolContractParameters::try_from(c.pool_contract_parameters)?;
        let pool_contract_prefix = pool_contract_parameters.p2s.network();

        let refresh_contract_parameters =
            RefreshContractParameters::try_from(c.refresh_contract_parameters)?;
        let refresh_contract_prefix = refresh_contract_parameters.p2s.network();

        let update_contract_parameters =
            UpdateContractParameters::try_from(c.update_contract_parameters)?;
        let update_contract_prefix = update_contract_parameters.p2s.network();

        let ballot_token_owner_address = AddressEncoder::unchecked_parse_network_address_from_str(
            &c.addresses.ballot_token_owner_address,
        )?;
        let network_prefix = ballot_token_owner_address.network();
        let ballot_contract_parameters =
            BallotContractParameters::try_from(c.ballot_contract_parameters)?;
        let ballot_contract_prefix = ballot_contract_parameters.p2s.network();

        let addresses = Addresses::try_from(c.addresses)?;
        let addresses_prefix = addresses.wallet_address_for_chain_transaction.network();

        let oracle_address =
            AddressEncoder::unchecked_parse_network_address_from_str(&c.oracle_address)?;

        if addresses_prefix == network_prefix
            && ballot_contract_prefix == network_prefix
            && update_contract_prefix == network_prefix
            && refresh_contract_prefix == network_prefix
            && oracle_contract_prefix == network_prefix
            && pool_contract_prefix == network_prefix
        {
            Ok(OracleConfig {
                node_ip: c.node_ip,
                node_port: c.node_port,
                node_api_key: c.node_api_key,
                base_fee: c.base_fee,
                log_level: c.log_level,
                core_api_port: c.core_api_port,
                oracle_address,
                data_point_source: c.data_point_source,
                data_point_source_custom_script: c.data_point_source_custom_script,
                oracle_contract_parameters,
                pool_contract_parameters,
                refresh_contract_parameters,
                update_contract_parameters,
                ballot_contract_parameters,
                token_ids: c.token_ids,
                addresses,
            })
        } else {
            Err(SerdeConversionError::NetworkPrefixesDiffer)
        }
    }
}

/// Used to (de)serialize `BootstrapConfig` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfigSerde {
    oracle_contract_parameters: OracleContractParametersSerde,
    refresh_contract_parameters: RefreshContractParametersSerde,
    pool_contract_parameters: PoolContractParametersSerde,
    update_contract_parameters: UpdateContractParametersSerde,
    ballot_contract_parameters: BallotContractParametersSerde,
    tokens_to_mint: TokensToMint,
    node_ip: String,
    node_port: u16,
    node_api_key: String,
    core_api_port: u16,
    data_point_source: Option<PredefinedDataPointSource>,
    data_point_source_custom_script: Option<String>,
    addresses: AddressesSerde,
    oracle_address: String,
    base_fee: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AddressesSerde {
    wallet_address_for_chain_transaction: String,
    pub ballot_token_owner_address: String,
}

impl From<Addresses> for AddressesSerde {
    fn from(addresses: Addresses) -> Self {
        AddressesSerde {
            wallet_address_for_chain_transaction: addresses
                .wallet_address_for_chain_transaction
                .to_base58(),
            ballot_token_owner_address: addresses.ballot_token_owner_address.to_base58(),
        }
    }
}

impl TryFrom<AddressesSerde> for Addresses {
    type Error = SerdeConversionError;
    fn try_from(addresses: AddressesSerde) -> Result<Self, Self::Error> {
        let wallet_address_for_chain_transaction =
            AddressEncoder::unchecked_parse_network_address_from_str(
                &addresses.wallet_address_for_chain_transaction,
            )?;
        let ballot_token_owner_address = AddressEncoder::unchecked_parse_network_address_from_str(
            &addresses.ballot_token_owner_address,
        )?;
        if ballot_token_owner_address.network() == wallet_address_for_chain_transaction.network() {
            Ok(Addresses {
                wallet_address_for_chain_transaction,
                ballot_token_owner_address,
            })
        } else {
            Err(SerdeConversionError::NetworkPrefixesDiffer)
        }
    }
}

impl From<BootstrapConfig> for BootstrapConfigSerde {
    fn from(c: BootstrapConfig) -> Self {
        BootstrapConfigSerde {
            oracle_contract_parameters: c.oracle_contract_parameters.into(),
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
            addresses: AddressesSerde::from(c.addresses),
            oracle_address: c.oracle_address.to_base58(),
            core_api_port: c.core_api_port,
            data_point_source: c.data_point_source,
            data_point_source_custom_script: c.data_point_source_custom_script,
            base_fee: c.base_fee,
        }
    }
}

impl TryFrom<BootstrapConfigSerde> for BootstrapConfig {
    type Error = SerdeConversionError;

    fn try_from(c: BootstrapConfigSerde) -> Result<Self, Self::Error> {
        let pool_contract_parameters =
            PoolContractParameters::try_from(c.pool_contract_parameters)?;
        let pool_contract_prefix = pool_contract_parameters.p2s.network();
        let refresh_contract_parameters =
            RefreshContractParameters::try_from(c.refresh_contract_parameters)?;
        let refresh_contract_prefix = refresh_contract_parameters.p2s.network();
        let update_contract_parameters =
            UpdateContractParameters::try_from(c.update_contract_parameters)?;
        let update_contract_prefix = update_contract_parameters.p2s.network();
        let ballot_contract_parameters =
            BallotContractParameters::try_from(c.ballot_contract_parameters)?;
        let ballot_contract_prefix = ballot_contract_parameters.p2s.network();
        let addresses = Addresses::try_from(c.addresses)?;
        let addresses_prefix = addresses.ballot_token_owner_address.network();
        let oracle_contract_parameters =
            OracleContractParameters::try_from(c.oracle_contract_parameters)?;
        let oracle_address =
            AddressEncoder::unchecked_parse_network_address_from_str(&c.oracle_address)?;

        if pool_contract_prefix == addresses_prefix
            && refresh_contract_prefix == addresses_prefix
            && update_contract_prefix == addresses_prefix
            && ballot_contract_prefix == addresses_prefix
        {
            Ok(BootstrapConfig {
                oracle_contract_parameters,
                pool_contract_parameters,
                refresh_contract_parameters,
                update_contract_parameters,
                ballot_contract_parameters,
                tokens_to_mint: c.tokens_to_mint,
                node_ip: c.node_ip,
                node_port: c.node_port,
                node_api_key: c.node_api_key,
                addresses,
                oracle_address,
                core_api_port: c.core_api_port,
                data_point_source: c.data_point_source,
                data_point_source_custom_script: c.data_point_source_custom_script,
                base_fee: c.base_fee,
            })
        } else {
            Err(SerdeConversionError::NetworkPrefixesDiffer)
        }
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
            p2s: p.p2s.to_base58(),
            pool_nft_index: p.pool_nft_index,
        }
    }
}

impl TryFrom<OracleContractParametersSerde> for OracleContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: OracleContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s = AddressEncoder::unchecked_parse_network_address_from_str(&contract.p2s)?;

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
            p2s: p.p2s.to_base58(),
            refresh_nft_index: p.refresh_nft_index,
            update_nft_index: p.update_nft_index,
        }
    }
}

impl TryFrom<PoolContractParametersSerde> for PoolContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: PoolContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s = AddressEncoder::unchecked_parse_network_address_from_str(&contract.p2s)?;
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
    min_data_points: i32,
    buffer_index: usize,
    buffer_length: i32,
    max_deviation_percent_index: usize,
    max_deviation_percent: i32,
    epoch_length_index: usize,
    epoch_length: i32,
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

impl TryFrom<RefreshContractParametersSerde> for RefreshContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: RefreshContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s = AddressEncoder::unchecked_parse_network_address_from_str(&contract.p2s)?;
        Ok(RefreshContractParameters {
            p2s,
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
            p2s: c.p2s.to_base58(),
            min_storage_rent_index: c.min_storage_rent_index,
            min_storage_rent: c.min_storage_rent,
            update_nft_index: c.update_nft_index,
        }
    }
}

impl TryFrom<BallotContractParametersSerde> for BallotContractParameters {
    type Error = AddressEncoderError;
    fn try_from(contract: BallotContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s = AddressEncoder::unchecked_parse_network_address_from_str(&contract.p2s)?;
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

impl TryFrom<UpdateContractParametersSerde> for UpdateContractParameters {
    type Error = AddressEncoderError;

    fn try_from(contract: UpdateContractParametersSerde) -> Result<Self, Self::Error> {
        let p2s = AddressEncoder::unchecked_parse_network_address_from_str(&contract.p2s)?;
        Ok(UpdateContractParameters {
            p2s,
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

/// The network prefix of the 2nd element is the one in use by the existing oracle pool.
impl TryFrom<(UpdateBootstrapConfigSerde, NetworkPrefix)> for UpdateBootstrapConfig {
    type Error = SerdeConversionError;
    fn try_from(
        (config_serde, existing_network_prefix): (UpdateBootstrapConfigSerde, NetworkPrefix),
    ) -> Result<UpdateBootstrapConfig, Self::Error> {
        // Here we collect the network prefixes of any contract updates, to check for equality with
        // existing_network_prefix.
        let mut prefixes = vec![];

        let pool_contract_parameters: Option<PoolContractParameters> = config_serde
            .pool_contract_parameters
            .map(|r| r.try_into())
            .transpose()?;
        if let Some(p) = &pool_contract_parameters {
            prefixes.push(p.p2s.network());
        }

        let refresh_contract_parameters: Option<RefreshContractParameters> = config_serde
            .refresh_contract_parameters
            .map(|r| r.try_into())
            .transpose()?;
        if let Some(p) = &refresh_contract_parameters {
            prefixes.push(p.p2s.network());
        }

        let update_contract_parameters: Option<UpdateContractParameters> = config_serde
            .update_contract_parameters
            .map(|r| r.try_into())
            .transpose()?;
        if let Some(p) = &update_contract_parameters {
            prefixes.push(p.p2s.network());
        }

        let addresses = Addresses::try_from(c.addresses)?;
        let addresses_prefix = addresses.ballot_token_owner_address.network();
        prefixes.push(addresses_prefix);

        for p in prefixes {
            if p != existing_network_prefix {
                return Err(SerdeConversionError::NetworkPrefixesDiffer);
            }
        }
        Ok(UpdateBootstrapConfig {
            pool_contract_parameters,
            refresh_contract_parameters,
            update_contract_parameters,
            tokens_to_mint: config_serde.tokens_to_mint,
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
