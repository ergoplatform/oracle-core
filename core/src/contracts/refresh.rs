use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use thiserror::Error;

use crate::box_kind::RefreshBoxWrapperInputs;

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
    pub fn new(inputs: RefreshContractInputs) -> Result<Self, RefreshContractError> {
        let ergo_tree = inputs
            .contract_parameters
            .p2s
            .script()?
            .with_constant(
                inputs.contract_parameters.pool_nft_index,
                inputs.pool_nft_token_id.clone().into(),
            )
            .map_err(RefreshContractError::ErgoTreeConstant)?
            .with_constant(
                inputs.contract_parameters.oracle_token_id_index,
                inputs.oracle_token_id.clone().into(),
            )
            .map_err(RefreshContractError::ErgoTreeConstant)?;

        let contract = Self::from_ergo_tree(ergo_tree, inputs)?;
        Ok(contract)
    }

    // TODO: switch to `TryFrom`
    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: RefreshContractInputs,
    ) -> Result<Self, RefreshContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        let parameters = inputs.contract_parameters;
        let pool_nft_token_id = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| RefreshContractError::NoPoolNftId)?
            .ok_or(RefreshContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        if pool_nft_token_id != *inputs.pool_nft_token_id {
            return Err(RefreshContractError::PoolNftTokenIdDiffers {
                expected: inputs.pool_nft_token_id.clone(),
                actual: pool_nft_token_id,
            });
        }

        let oracle_token_id = ergo_tree
            .get_constant(parameters.oracle_token_id_index)
            .map_err(|_| RefreshContractError::NoOracleTokenId)?
            .ok_or(RefreshContractError::NoOracleTokenId)?
            .try_extract_into::<TokenId>()?;
        if oracle_token_id != *inputs.oracle_token_id {
            return Err(RefreshContractError::OracleTokenIdDiffers {
                expected: inputs.oracle_token_id.clone(),
                actual: oracle_token_id,
            });
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

pub struct RefreshContractInputs<'a> {
    pub contract_parameters: &'a RefreshContractParameters,
    pub oracle_token_id: &'a TokenId,
    pub pool_nft_token_id: &'a TokenId,
}

impl<'a> From<RefreshBoxWrapperInputs<'a>> for RefreshContractInputs<'a> {
    fn from(b: RefreshBoxWrapperInputs<'a>) -> Self {
        RefreshContractInputs {
            contract_parameters: b.contract_parameters,
            oracle_token_id: b.oracle_token_id,
            pool_nft_token_id: b.pool_nft_token_id,
        }
    }
}

#[derive(Debug, Clone)]
/// Parameters for the pool contract
pub struct RefreshContractParameters {
    pub p2s: Address,
    pub pool_nft_index: usize,
    pub oracle_token_id_index: usize,
    pub min_data_points_index: usize,
    pub min_data_points: u64,
    pub buffer_index: usize,
    pub buffer_length: u64,
    pub max_deviation_percent_index: usize,
    pub max_deviation_percent: u64,
    pub epoch_length_index: usize,
    pub epoch_length: u64,
}

#[cfg(test)]
mod tests {

    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = RefreshContractParameters::default();
        let token_ids = generate_token_ids();
        let inputs = RefreshContractInputs {
            contract_parameters: &parameters,
            oracle_token_id: &token_ids.oracle_token_id,
            pool_nft_token_id: &token_ids.pool_nft_token_id,
        };
        let c = RefreshContract::new(inputs).unwrap();
        assert_eq!(c.pool_nft_token_id(), token_ids.pool_nft_token_id,);
        assert_eq!(c.oracle_token_id(), token_ids.oracle_token_id,);
        assert_eq!(c.min_data_points() as u64, parameters.min_data_points);
        assert_eq!(c.buffer() as u64, parameters.buffer_length);
        assert_eq!(
            c.max_deviation_percent() as u64,
            parameters.max_deviation_percent
        );
        assert_eq!(c.epoch_length() as u64, parameters.epoch_length);
    }
}
