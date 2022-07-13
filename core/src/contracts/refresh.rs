use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

#[derive(Clone)]
pub struct RefreshContract {
    ergo_tree: ErgoTree,
    pool_nft_index: usize,
    oracle_token_id_index: usize,
    min_data_points_index: usize,
    buffer_index: usize,
    max_deviation_percent_index: usize,
    epoch_length_index: usize,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum RefreshContractError {
    #[error("refresh contract: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error(
        "refresh contract: unexpected `pool NFT` token id. Expected {expected:?}, got {actual:?}"
    )]
    PoolNftTokenIdDiffers { expected: TokenId, actual: TokenId },
    #[error("refresh contract: failed to get oracle token id from constants")]
    NoOracleTokenId,
    #[error(
        "refresh contract: unexpected `oracle` token id. Expected {expected:?}, got {actual:?}"
    )]
    OracleTokenIdDiffers { expected: TokenId, actual: TokenId },
    #[error("refresh contract: failed to get min data points from constants")]
    NoMinDataPoints,
    #[error(
        "refresh contract: unexpected `min data points` value from constants. Expected {expected}, got {actual}"
    )]
    MinDataPointsDiffers { expected: u64, actual: u64 },
    #[error(
        "refresh contract: unexpected `buffer length` value from constants. Expected {expected}, got {actual}"
    )]
    BufferLengthDiffers { expected: u64, actual: u64 },
    #[error(
        "refresh contract: unexpected `max deviation percentage` value from constants. Expected {expected}, got {actual}"
    )]
    MaxDeviationPercentDiffers { expected: u64, actual: u64 },
    #[error(
        "refresh contract: unexpected `epoch length` value from constants. Expected {expected}, got {actual}"
    )]
    EpochLengthDiffers { expected: u64, actual: u64 },
    #[error("refresh contract: failed to get buffer from constants")]
    NoBuffer,
    #[error("refresh contract: failed to get max deviation percent from constants")]
    NoMaxDeviationPercent,
    #[error("refresh contract: failed to get epoch length from constants")]
    NoEpochLength,
    #[error("refresh contract: sigma parsing error {0}")]
    SigmaParsing(#[from] SigmaParsingError),
    #[error("refresh contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("refresh contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(#[from] TryExtractFromError),
}

impl RefreshContract {
    pub fn new(parameters: &RefreshContractParameters) -> Result<Self, RefreshContractError> {
        let ergo_tree = parameters
            .p2s
            .address()
            .script()?
            .with_constant(
                parameters.pool_nft_index,
                parameters.pool_nft_token_id.clone().into(),
            )
            .map_err(RefreshContractError::ErgoTreeConstant)?
            .with_constant(
                parameters.oracle_token_id_index,
                parameters.oracle_token_id.clone().into(),
            )
            .map_err(RefreshContractError::ErgoTreeConstant)?;

        let contract = Self::from_ergo_tree(ergo_tree, parameters)?;
        Ok(contract)
    }

    // TODO: switch to `TryFrom`
    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &RefreshContractParameters,
    ) -> Result<Self, RefreshContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        let pool_nft_token_id = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| RefreshContractError::NoPoolNftId)?
            .ok_or(RefreshContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>();
        match pool_nft_token_id {
            Ok(token_id) => {
                if token_id != parameters.pool_nft_token_id {
                    return Err(RefreshContractError::PoolNftTokenIdDiffers {
                        expected: parameters.pool_nft_token_id.clone(),
                        actual: token_id,
                    });
                }
            }
            Err(e) => {
                return Err(RefreshContractError::TryExtractFrom(e));
            }
        }

        let oracle_token_id = ergo_tree
            .get_constant(parameters.oracle_token_id_index)
            .map_err(|_| RefreshContractError::NoOracleTokenId)?
            .ok_or(RefreshContractError::NoOracleTokenId)?
            .try_extract_into::<TokenId>();
        match oracle_token_id {
            Ok(token_id) => {
                if token_id != parameters.oracle_token_id {
                    return Err(RefreshContractError::OracleTokenIdDiffers {
                        expected: parameters.oracle_token_id.clone(),
                        actual: token_id,
                    });
                }
            }
            Err(e) => {
                return Err(RefreshContractError::TryExtractFrom(e));
            }
        }

        let min_data_points = ergo_tree
            .get_constant(parameters.min_data_points_index)
            .map_err(|_| RefreshContractError::NoMinDataPoints)?
            .ok_or(RefreshContractError::NoMinDataPoints)?
            .try_extract_into::<i32>()?;
        if min_data_points as u64 != parameters.min_data_points {
            return Err(RefreshContractError::MinDataPointsDiffers {
                expected: parameters.min_data_points,
                actual: min_data_points as u64,
            });
        }

        let buffer_length = ergo_tree
            .get_constant(parameters.buffer_index)
            .map_err(|_| RefreshContractError::NoBuffer)?
            .ok_or(RefreshContractError::NoBuffer)?
            .try_extract_into::<i32>()? as u64;
        if buffer_length != parameters.buffer_length {
            return Err(RefreshContractError::BufferLengthDiffers {
                expected: parameters.buffer_length,
                actual: buffer_length,
            });
        }

        let max_deviation_percent = ergo_tree
            .get_constant(parameters.max_deviation_percent_index)
            .map_err(|_| RefreshContractError::NoMaxDeviationPercent)?
            .ok_or(RefreshContractError::NoMaxDeviationPercent)?
            .try_extract_into::<i32>()? as u64;
        if max_deviation_percent != parameters.max_deviation_percent {
            return Err(RefreshContractError::MaxDeviationPercentDiffers {
                expected: parameters.max_deviation_percent,
                actual: max_deviation_percent,
            });
        }

        let epoch_length = ergo_tree
            .get_constant(parameters.epoch_length_index)
            .map_err(|_| RefreshContractError::NoEpochLength)?
            .ok_or(RefreshContractError::NoEpochLength)?
            .try_extract_into::<i32>()? as u64;
        if epoch_length != parameters.epoch_length {
            return Err(RefreshContractError::EpochLengthDiffers {
                expected: parameters.epoch_length,
                actual: epoch_length,
            });
        }

        Ok(Self {
            ergo_tree,
            pool_nft_index: parameters.pool_nft_index,
            oracle_token_id_index: parameters.oracle_token_id_index,
            min_data_points_index: parameters.min_data_points_index,
            buffer_index: parameters.buffer_index,
            max_deviation_percent_index: parameters.max_deviation_percent_index,
            epoch_length_index: parameters.epoch_length_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn epoch_length(&self) -> u32 {
        self.ergo_tree
            .get_constant(self.epoch_length_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn buffer(&self) -> u32 {
        self.ergo_tree
            .get_constant(self.buffer_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn min_data_points(&self) -> u32 {
        self.ergo_tree
            .get_constant(self.min_data_points_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn max_deviation_percent(&self) -> u32 {
        self.ergo_tree
            .get_constant(self.max_deviation_percent_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn oracle_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.oracle_token_id_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn pool_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.pool_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    try_from = "RefreshContractParametersYaml",
    into = "RefreshContractParametersYaml"
)]
/// Parameters for the pool contract
pub struct RefreshContractParameters {
    pub p2s: NetworkAddress,
    pub refresh_nft_token_id: TokenId,
    pub pool_nft_index: usize,
    pub pool_nft_token_id: TokenId,
    pub oracle_token_id_index: usize,
    pub oracle_token_id: TokenId,
    pub min_data_points_index: usize,
    pub min_data_points: u64,
    pub buffer_index: usize,
    pub buffer_length: u64,
    pub max_deviation_percent_index: usize,
    pub max_deviation_percent: u64,
    pub epoch_length_index: usize,
    pub epoch_length: u64,
}

/// Used to (de)serialize `RefreshContractParameters` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshContractParametersYaml {
    p2s: String,
    on_mainnet: bool,
    refresh_nft_token_id: TokenId,
    pool_nft_index: usize,
    pool_nft_token_id: TokenId,
    oracle_token_id_index: usize,
    oracle_token_id: TokenId,
    min_data_points_index: usize,
    min_data_points: u64,
    buffer_index: usize,
    buffer_length: u64,
    max_deviation_percent_index: usize,
    max_deviation_percent: u64,
    epoch_length_index: usize,
    epoch_length: u64,
}

impl TryFrom<RefreshContractParametersYaml> for RefreshContractParameters {
    type Error = AddressEncoderError;

    fn try_from(p: RefreshContractParametersYaml) -> Result<Self, Self::Error> {
        let prefix = if p.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        let address = AddressEncoder::new(prefix).parse_address_from_str(&p.p2s)?;
        Ok(RefreshContractParameters {
            p2s: NetworkAddress::new(prefix, &address),
            refresh_nft_token_id: p.refresh_nft_token_id,
            pool_nft_index: p.pool_nft_index,
            pool_nft_token_id: p.pool_nft_token_id,
            oracle_token_id_index: p.oracle_token_id_index,
            oracle_token_id: p.oracle_token_id,
            min_data_points_index: p.min_data_points_index,
            min_data_points: p.min_data_points,
            buffer_index: p.buffer_index,
            buffer_length: p.buffer_length,
            max_deviation_percent_index: p.max_deviation_percent_index,
            max_deviation_percent: p.max_deviation_percent,
            epoch_length_index: p.epoch_length_index,
            epoch_length: p.epoch_length,
        })
    }
}

impl From<RefreshContractParameters> for RefreshContractParametersYaml {
    fn from(p: RefreshContractParameters) -> Self {
        RefreshContractParametersYaml {
            p2s: p.p2s.to_base58(),
            on_mainnet: p.p2s.network() == NetworkPrefix::Mainnet,
            refresh_nft_token_id: p.refresh_nft_token_id,
            pool_nft_index: p.pool_nft_index,
            pool_nft_token_id: p.pool_nft_token_id,
            oracle_token_id_index: p.oracle_token_id_index,
            oracle_token_id: p.oracle_token_id,
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

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::make_refresh_contract_parameters;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_refresh_contract_parameters();
        let c = RefreshContract::new(&parameters).unwrap();
        assert_eq!(c.pool_nft_token_id(), parameters.pool_nft_token_id,);
        assert_eq!(c.oracle_token_id(), parameters.oracle_token_id,);
        assert_eq!(c.min_data_points() as u64, parameters.min_data_points);
        assert_eq!(c.buffer() as u64, parameters.buffer_length);
        assert_eq!(
            c.max_deviation_percent() as u64,
            parameters.max_deviation_percent
        );
        assert_eq!(c.epoch_length() as u64, parameters.epoch_length);
    }
}
