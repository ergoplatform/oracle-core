//! Types to allow oracle configuration to convert to and from Serde.

use std::convert::{TryFrom, TryInto};

use derive_more::From;
use ergo_lib::{
    ergo_chain_types::Digest32,
    ergotree_ir::chain::{address::AddressEncoderError, ergo_box::box_value::BoxValueError},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    box_kind::{
        BallotBoxWrapperInputs, OracleBoxWrapperInputs, PoolBoxWrapperInputs,
        RefreshBoxWrapperInputs, UpdateBoxWrapperInputs,
    },
    cli_commands::{
        bootstrap::{BootstrapConfig, TokensToMint},
        prepare_update::{UpdateBootstrapConfig, UpdateTokensToMint},
    },
    contracts::{
        ballot::{BallotContractParameters, BallotContractParametersError},
        oracle::{OracleContractParameters, OracleContractParametersError},
        pool::{PoolContractParameters, PoolContractParametersError},
        refresh::{
            RefreshContractParameters, RefreshContractParametersError,
            RefreshContractParametersInputs,
        },
        update::{UpdateContractParameters, UpdateContractParametersError},
    },
    pool_config::{PoolConfig, PoolConfigError, TokenIds},
    spec_token::TokenIdKind,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PoolConfigSerde {
    oracle_contract_parameters: OracleContractParametersSerde,
    pool_contract_parameters: PoolContractParametersSerde,
    refresh_contract_parameters: RefreshContractParametersSerde,
    update_contract_parameters: UpdateContractParametersSerde,
    ballot_contract_parameters: BallotContractParametersSerde,
    token_ids: TokenIds,
}

#[derive(Debug, Error, From)]
pub enum SerdeConversionError {
    #[error("Serde conversion error: AddressEncoder {0}")]
    AddressEncoder(AddressEncoderError),
    #[error("Pool config error: {0}")]
    PoolConfigError(PoolConfigError),
    #[error("Base16 decode error: {0}")]
    DecodeError(base16::DecodeError),
    #[error("Ballot contract parameter error: {0}")]
    BallotContractParameters(BallotContractParametersError),
    #[error("Oracle contract parameter error: {0}")]
    OracleContractParameters(OracleContractParametersError),
    #[error("Pool contract parameter error: {0}")]
    PoolContractParameters(PoolContractParametersError),
    #[error("Refresh contract parameter error: {0}")]
    RefreshContractParameters(RefreshContractParametersError),
    #[error("Update contract parameter error: {0}")]
    UpdateContractParameters(UpdateContractParametersError),
    #[error("BoxValueError: {0}")]
    BoxValueError(BoxValueError),
}

impl From<PoolConfig> for PoolConfigSerde {
    fn from(c: PoolConfig) -> Self {
        let oracle_contract_parameters = OracleContractParametersSerde::from(
            c.oracle_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .clone(),
        );
        let pool_contract_parameters = PoolContractParametersSerde::from(
            c.pool_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .clone(),
        );
        let refresh_contract_parameters = RefreshContractParametersSerde::from(
            c.refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .clone(),
        );
        let ballot_contract_parameters = BallotContractParametersSerde::from(
            c.ballot_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .clone(),
        );
        let update_contract_parameters = UpdateContractParametersSerde::from(
            c.update_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .clone(),
        );

        PoolConfigSerde {
            oracle_contract_parameters,
            pool_contract_parameters,
            refresh_contract_parameters,
            ballot_contract_parameters,
            update_contract_parameters,
            token_ids: c.token_ids,
        }
    }
}

impl TryFrom<PoolConfigSerde> for PoolConfig {
    type Error = SerdeConversionError;
    fn try_from(c: PoolConfigSerde) -> Result<Self, Self::Error> {
        let oracle_contract_parameters = OracleContractParameters::checked_load(
            base16::decode(c.oracle_contract_parameters.ergo_tree_bytes.as_str())?,
            c.oracle_contract_parameters.pool_nft_index,
            c.oracle_contract_parameters.min_storage_rent_index,
            c.oracle_contract_parameters.min_storage_rent.try_into()?,
        )?;

        let oracle_box_wrapper_inputs = OracleBoxWrapperInputs::checked_load(
            oracle_contract_parameters.clone(),
            c.token_ids.pool_nft_token_id.clone(),
            c.token_ids.oracle_token_id.clone(),
            c.token_ids.reward_token_id.clone(),
        )
        .map_err(PoolConfigError::from)?;

        let pool_contract_parameters = PoolContractParameters::checked_load(
            base16::decode(c.pool_contract_parameters.ergo_tree_bytes.as_str())?,
            c.pool_contract_parameters.refresh_nft_index,
            c.pool_contract_parameters.update_nft_index,
        )?;

        let refresh_contract_parameters =
            RefreshContractParameters::checked_load(RefreshContractParametersInputs {
                ergo_tree_bytes: base16::decode(
                    c.refresh_contract_parameters.ergo_tree_bytes.as_str(),
                )?,
                pool_nft_index: c.refresh_contract_parameters.pool_nft_index,
                oracle_token_id_index: c.refresh_contract_parameters.oracle_token_id_index,
                min_data_points_index: c.refresh_contract_parameters.min_data_points_index,
                min_data_points: c.refresh_contract_parameters.min_data_points,
                buffer_length_index: c.refresh_contract_parameters.buffer_length_index,
                buffer_length: c.refresh_contract_parameters.buffer_length,
                max_deviation_percent_index: c
                    .refresh_contract_parameters
                    .max_deviation_percent_index,
                max_deviation_percent: c.refresh_contract_parameters.max_deviation_percent,
                epoch_length_index: c.refresh_contract_parameters.epoch_length_index,
                epoch_length: c.refresh_contract_parameters.epoch_length,
            })?;

        let update_contract_parameters = UpdateContractParameters::checked_load(
            base16::decode(c.update_contract_parameters.ergo_tree_bytes.as_str())?,
            c.update_contract_parameters.pool_nft_index,
            c.update_contract_parameters.ballot_token_index,
            c.update_contract_parameters.min_votes_index,
            c.update_contract_parameters.min_votes,
        )?;

        let ballot_contract_parameters = BallotContractParameters::checked_load(
            base16::decode(c.ballot_contract_parameters.ergo_tree_bytes.as_str())?,
            c.ballot_contract_parameters.min_storage_rent.try_into()?,
            c.ballot_contract_parameters.min_storage_rent_index,
            c.ballot_contract_parameters.update_nft_index,
        )?;

        let refresh_box_wrapper_inputs = RefreshBoxWrapperInputs::checked_load(
            refresh_contract_parameters.clone(),
            c.token_ids.oracle_token_id.clone(),
            c.token_ids.pool_nft_token_id.clone(),
            c.token_ids.refresh_nft_token_id.clone(),
        )
        .map_err(PoolConfigError::from)?;

        let pool_box_wrapper_inputs = PoolBoxWrapperInputs::checked_load(
            pool_contract_parameters.clone(),
            c.token_ids.refresh_nft_token_id.clone(),
            c.token_ids.update_nft_token_id.clone(),
            c.token_ids.pool_nft_token_id.clone(),
            c.token_ids.reward_token_id.clone(),
        )
        .map_err(PoolConfigError::from)?;

        let update_box_wrapper_inputs = UpdateBoxWrapperInputs::checked_load(
            update_contract_parameters.clone(),
            c.token_ids.pool_nft_token_id.clone(),
            c.token_ids.ballot_token_id.clone(),
            c.token_ids.update_nft_token_id.clone(),
        )
        .map_err(PoolConfigError::from)?;

        let ballot_box_wrapper_inputs = BallotBoxWrapperInputs::checked_load(
            ballot_contract_parameters.clone(),
            c.token_ids.ballot_token_id.clone(),
            c.token_ids.update_nft_token_id.clone(),
        )
        .map_err(PoolConfigError::from)?;

        Ok(PoolConfig {
            oracle_box_wrapper_inputs,
            pool_box_wrapper_inputs,
            refresh_box_wrapper_inputs,
            update_box_wrapper_inputs,
            ballot_box_wrapper_inputs,
            token_ids: c.token_ids,
        })
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
        }
    }
}

impl TryFrom<BootstrapConfigSerde> for BootstrapConfig {
    type Error = SerdeConversionError;

    fn try_from(c: BootstrapConfigSerde) -> Result<Self, Self::Error> {
        let pool_contract_parameters = PoolContractParameters::checked_load(
            base16::decode(c.pool_contract_parameters.ergo_tree_bytes.as_str())?,
            c.pool_contract_parameters.refresh_nft_index,
            c.pool_contract_parameters.update_nft_index,
        )?;
        let refresh_contract_parameters =
            RefreshContractParameters::build_with(RefreshContractParametersInputs {
                ergo_tree_bytes: base16::decode(
                    c.refresh_contract_parameters.ergo_tree_bytes.as_str(),
                )?,
                pool_nft_index: c.refresh_contract_parameters.pool_nft_index,
                oracle_token_id_index: c.refresh_contract_parameters.oracle_token_id_index,
                min_data_points_index: c.refresh_contract_parameters.min_data_points_index,
                min_data_points: c.refresh_contract_parameters.min_data_points,
                buffer_length_index: c.refresh_contract_parameters.buffer_length_index,
                buffer_length: c.refresh_contract_parameters.buffer_length,
                max_deviation_percent_index: c
                    .refresh_contract_parameters
                    .max_deviation_percent_index,
                max_deviation_percent: c.refresh_contract_parameters.max_deviation_percent,
                epoch_length_index: c.refresh_contract_parameters.epoch_length_index,
                epoch_length: c.refresh_contract_parameters.epoch_length,
            })?;
        let update_contract_parameters = UpdateContractParameters::build_with(
            base16::decode(c.update_contract_parameters.ergo_tree_bytes.as_str())?,
            c.update_contract_parameters.pool_nft_index,
            c.update_contract_parameters.ballot_token_index,
            c.update_contract_parameters.min_votes_index,
            c.update_contract_parameters.min_votes,
        )?;
        let ballot_contract_parameters = BallotContractParameters::build_with(
            base16::decode(c.ballot_contract_parameters.ergo_tree_bytes.as_str())?,
            c.ballot_contract_parameters.min_storage_rent_index,
            c.ballot_contract_parameters.min_storage_rent.try_into()?,
            c.ballot_contract_parameters.update_nft_index,
        )?;
        let oracle_contract_parameters = OracleContractParameters::build_with(
            base16::decode(c.oracle_contract_parameters.ergo_tree_bytes.as_str())?,
            c.oracle_contract_parameters.pool_nft_index,
            c.oracle_contract_parameters.min_storage_rent_index,
            c.oracle_contract_parameters.min_storage_rent.try_into()?,
        )?;

        Ok(BootstrapConfig {
            oracle_contract_parameters,
            pool_contract_parameters,
            refresh_contract_parameters,
            update_contract_parameters,
            ballot_contract_parameters,
            tokens_to_mint: c.tokens_to_mint,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleContractParametersSerde {
    ergo_tree_bytes: String,
    pool_nft_index: usize,
    min_storage_rent_index: usize,
    min_storage_rent: u64,
}

impl From<OracleContractParameters> for OracleContractParametersSerde {
    fn from(p: OracleContractParameters) -> Self {
        OracleContractParametersSerde {
            ergo_tree_bytes: base16::encode_lower(p.ergo_tree_bytes().as_slice()),
            pool_nft_index: p.pool_nft_index,
            min_storage_rent_index: p.min_storage_rent_index,
            min_storage_rent: *p.min_storage_rent.as_u64(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoolContractParametersSerde {
    ergo_tree_bytes: String,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

impl From<PoolContractParameters> for PoolContractParametersSerde {
    fn from(p: PoolContractParameters) -> Self {
        PoolContractParametersSerde {
            ergo_tree_bytes: base16::encode_lower(p.ergo_tree_bytes().as_slice()),
            refresh_nft_index: p.refresh_nft_index(),
            update_nft_index: p.update_nft_index(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshContractParametersSerde {
    ergo_tree_bytes: String,
    pool_nft_index: usize,
    oracle_token_id_index: usize,
    min_data_points_index: usize,
    min_data_points: i32,
    buffer_length_index: usize,
    buffer_length: i32,
    max_deviation_percent_index: usize,
    max_deviation_percent: i32,
    epoch_length_index: usize,
    epoch_length: i32,
}

impl From<RefreshContractParameters> for RefreshContractParametersSerde {
    fn from(p: RefreshContractParameters) -> Self {
        RefreshContractParametersSerde {
            ergo_tree_bytes: base16::encode_lower(p.ergo_tree_bytes().as_slice()),
            pool_nft_index: p.pool_nft_index(),
            oracle_token_id_index: p.oracle_token_id_index(),
            min_data_points_index: p.min_data_points_index(),
            min_data_points: p.min_data_points(),
            buffer_length_index: p.buffer_length_index(),
            buffer_length: p.buffer_length(),
            max_deviation_percent_index: p.max_deviation_percent_index(),
            max_deviation_percent: p.max_deviation_percent(),
            epoch_length_index: p.epoch_length_index(),
            epoch_length: p.epoch_length(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BallotContractParametersSerde {
    ergo_tree_bytes: String,
    min_storage_rent_index: usize,
    min_storage_rent: u64,
    update_nft_index: usize,
}

impl From<BallotContractParameters> for BallotContractParametersSerde {
    fn from(c: BallotContractParameters) -> Self {
        BallotContractParametersSerde {
            ergo_tree_bytes: base16::encode_lower(c.ergo_tree_bytes().as_slice()),
            min_storage_rent_index: c.min_storage_rent_index(),
            min_storage_rent: *c.min_storage_rent().as_u64(),
            update_nft_index: c.update_nft_index(),
        }
    }
}

/// Used to (de)serialize `OracleContractParameters` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateContractParametersSerde {
    ergo_tree_bytes: String,
    pool_nft_index: usize,
    ballot_token_index: usize,
    min_votes_index: usize,
    min_votes: u64,
}

impl From<UpdateContractParameters> for UpdateContractParametersSerde {
    fn from(p: UpdateContractParameters) -> Self {
        UpdateContractParametersSerde {
            ergo_tree_bytes: base16::encode_lower(p.ergo_tree_bytes().as_slice()),
            pool_nft_index: p.pool_nft_index(),
            ballot_token_index: p.ballot_token_index(),
            min_votes_index: p.min_votes_index(),
            min_votes: p.min_votes(),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct UpdateBootstrapConfigSerde {
    pool_contract_parameters: Option<PoolContractParametersSerde>,
    refresh_contract_parameters: Option<RefreshContractParametersSerde>,
    update_contract_parameters: Option<UpdateContractParametersSerde>,
    tokens_to_mint: UpdateTokensToMint,
}

impl TryFrom<UpdateBootstrapConfigSerde> for UpdateBootstrapConfig {
    type Error = SerdeConversionError;
    fn try_from(
        config_serde: UpdateBootstrapConfigSerde,
    ) -> Result<UpdateBootstrapConfig, Self::Error> {
        let pool_contract_parameters = if let Some(c) = config_serde.pool_contract_parameters {
            Some(PoolContractParameters::checked_load(
                base16::decode(c.ergo_tree_bytes.as_str())?,
                c.refresh_nft_index,
                c.update_nft_index,
            )?)
        } else {
            None
        };

        let refresh_contract_parameters = if let Some(c) = config_serde.refresh_contract_parameters
        {
            Some(RefreshContractParameters::build_with(
                RefreshContractParametersInputs {
                    ergo_tree_bytes: base16::decode(c.ergo_tree_bytes.as_str())?,
                    pool_nft_index: c.pool_nft_index,
                    oracle_token_id_index: c.oracle_token_id_index,
                    min_data_points_index: c.min_data_points_index,
                    min_data_points: c.min_data_points,
                    buffer_length_index: c.buffer_length_index,
                    buffer_length: c.buffer_length,
                    max_deviation_percent_index: c.max_deviation_percent_index,
                    max_deviation_percent: c.max_deviation_percent,
                    epoch_length_index: c.epoch_length_index,
                    epoch_length: c.epoch_length,
                },
            )?)
        } else {
            None
        };

        let update_contract_parameters = if let Some(c) = config_serde.update_contract_parameters {
            Some(UpdateContractParameters::build_with(
                base16::decode(c.ergo_tree_bytes.as_str())?,
                c.pool_nft_index,
                c.ballot_token_index,
                c.min_votes_index,
                c.min_votes,
            )?)
        } else {
            None
        };

        Ok(UpdateBootstrapConfig {
            pool_contract_parameters,
            refresh_contract_parameters,
            update_contract_parameters,
            tokens_to_mint: config_serde.tokens_to_mint,
        })
    }
}

pub(crate) fn token_id_as_base16_string<S, T: TokenIdKind>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&String::from(value.token_id()))
}

pub(crate) fn token_id_from_base16<'de, D, T: TokenIdKind>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    // Interesting fact: `s` can't be of type `&str` otherwise we get the following error at
    // runtime:
    //   "invalid type: string ..., expected a borrowed string"
    let s: String = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(T::from_token_id_unchecked(
        Digest32::try_from(s)
            .map_err(serde::de::Error::custom)?
            .into(),
    ))
}
