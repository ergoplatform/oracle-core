use base16::DecodeError;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_lib::ergotree_ir::serialization::SigmaSerializationError;
use thiserror::Error;

use crate::oracle_types::EpochLength;
use crate::oracle_types::MinDatapoints;
use crate::spec_token::OracleTokenId;
use crate::spec_token::PoolTokenId;
use crate::spec_token::TokenIdKind;

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
    #[error("refresh contract: parameter error: {0}")]
    Parameters(RefreshContractParametersError),
    #[error(
        "refresh contract: unexpected `pool NFT` token id. Expected {expected:?}, got {actual:?}"
    )]
    PoolNftTokenIdDiffers {
        expected: PoolTokenId,
        actual: TokenId,
    },
    #[error(
        "refresh contract: unexpected `oracle` token id. Expected {expected:?}, got {actual:?}"
    )]
    OracleTokenIdDiffers {
        expected: OracleTokenId,
        actual: TokenId,
    },
    #[error("refresh contract: sigma parsing error {0}")]
    SigmaParsing(#[from] SigmaParsingError),
    #[error("refresh contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("refresh contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(#[from] TryExtractFromError),
    #[error("contract error: {1:?}, expected P2S: {0}")]
    WrappedWithExpectedP2SAddress(String, Box<Self>),
}

impl RefreshContract {
    pub fn checked_load(inputs: &RefreshContractInputs) -> Result<Self, RefreshContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs).map_err(|e| {
            let expected_base16 = Self::build_with(inputs)
                .unwrap()
                .ergo_tree
                .to_base16_bytes()
                .unwrap();
            RefreshContractError::WrappedWithExpectedP2SAddress(expected_base16, e.into())
        })?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: &RefreshContractInputs,
    ) -> Result<Self, RefreshContractError> {
        // dbg!(ergo_tree.get_constants().unwrap());

        let parameters = inputs.contract_parameters.clone();
        let pool_nft_token_id = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| {
                RefreshContractError::Parameters(RefreshContractParametersError::NoPoolNftId)
            })?
            .ok_or(RefreshContractError::Parameters(
                RefreshContractParametersError::NoPoolNftId,
            ))?
            .try_extract_into::<TokenId>()?;
        if pool_nft_token_id != inputs.pool_nft_token_id.token_id() {
            return Err(RefreshContractError::PoolNftTokenIdDiffers {
                expected: inputs.pool_nft_token_id.clone(),
                actual: pool_nft_token_id,
            });
        }

        let oracle_token_id = ergo_tree
            .get_constant(parameters.oracle_token_id_index)
            .map_err(|_| {
                RefreshContractError::Parameters(RefreshContractParametersError::NoOracleTokenId)
            })?
            .ok_or(RefreshContractError::Parameters(
                RefreshContractParametersError::NoOracleTokenId,
            ))?
            .try_extract_into::<TokenId>()?;
        if oracle_token_id != inputs.oracle_token_id.token_id() {
            return Err(RefreshContractError::OracleTokenIdDiffers {
                expected: inputs.oracle_token_id.clone(),
                actual: oracle_token_id,
            });
        }

        let min_data_points = ergo_tree
            .get_constant(parameters.min_data_points_index)
            .map_err(|_| {
                RefreshContractError::Parameters(RefreshContractParametersError::NoMinDataPoints)
            })?
            .ok_or(RefreshContractError::Parameters(
                RefreshContractParametersError::NoMinDataPoints,
            ))?
            .try_extract_into::<i32>()?;
        if min_data_points != parameters.min_data_points.0 {
            return Err(RefreshContractError::Parameters(
                RefreshContractParametersError::MinDataPointsDiffers {
                    expected: parameters.min_data_points.0,
                    actual: min_data_points,
                },
            ));
        }

        let buffer_length = ergo_tree
            .get_constant(parameters.buffer_length_index)
            .map_err(|_| {
                RefreshContractError::Parameters(RefreshContractParametersError::NoBufferLength)
            })?
            .ok_or(RefreshContractError::Parameters(
                RefreshContractParametersError::NoBufferLength,
            ))?
            .try_extract_into::<i32>()?;
        if buffer_length != parameters.buffer_length {
            return Err(RefreshContractError::Parameters(
                RefreshContractParametersError::BufferLengthDiffers {
                    expected: parameters.buffer_length,
                    actual: buffer_length,
                },
            ));
        }

        let max_deviation_percent = ergo_tree
            .get_constant(parameters.max_deviation_percent_index)
            .map_err(|_| {
                RefreshContractError::Parameters(
                    RefreshContractParametersError::NoMaxDeviationPercent,
                )
            })?
            .ok_or(RefreshContractError::Parameters(
                RefreshContractParametersError::NoMaxDeviationPercent,
            ))?
            .try_extract_into::<i32>()?;
        if max_deviation_percent != parameters.max_deviation_percent {
            return Err(RefreshContractError::Parameters(
                RefreshContractParametersError::MaxDeviationPercentDiffers {
                    expected: parameters.max_deviation_percent,
                    actual: max_deviation_percent,
                },
            ));
        }

        let epoch_length = EpochLength(
            ergo_tree
                .get_constant(parameters.epoch_length_index)
                .map_err(|_| {
                    RefreshContractError::Parameters(RefreshContractParametersError::NoEpochLength)
                })?
                .ok_or(RefreshContractError::Parameters(
                    RefreshContractParametersError::NoEpochLength,
                ))?
                .try_extract_into::<i32>()?,
        );
        if epoch_length != parameters.epoch_length {
            return Err(RefreshContractError::Parameters(
                RefreshContractParametersError::EpochLengthDiffers {
                    expected: parameters.epoch_length,
                    actual: epoch_length,
                },
            ));
        }

        Ok(Self {
            ergo_tree,
            pool_nft_index: parameters.pool_nft_index,
            oracle_token_id_index: parameters.oracle_token_id_index,
            min_data_points_index: parameters.min_data_points_index,
            buffer_index: parameters.buffer_length_index,
            max_deviation_percent_index: parameters.max_deviation_percent_index,
            epoch_length_index: parameters.epoch_length_index,
        })
    }

    fn build_with(inputs: &RefreshContractInputs) -> Result<Self, RefreshContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?
                .with_constant(
                    inputs.contract_parameters.pool_nft_index,
                    inputs.pool_nft_token_id.clone().token_id().into(),
                )
                .map_err(RefreshContractError::ErgoTreeConstant)?
                .with_constant(
                    inputs.contract_parameters.oracle_token_id_index,
                    inputs.oracle_token_id.clone().token_id().into(),
                )
                .map_err(RefreshContractError::ErgoTreeConstant)?
                .with_constant(
                    inputs.contract_parameters.min_data_points_index,
                    (inputs.contract_parameters.min_data_points.0).into(),
                )
                .map_err(RefreshContractError::ErgoTreeConstant)?
                .with_constant(
                    inputs.contract_parameters.buffer_length_index,
                    (inputs.contract_parameters.buffer_length).into(),
                )
                .map_err(RefreshContractError::ErgoTreeConstant)?
                .with_constant(
                    inputs.contract_parameters.max_deviation_percent_index,
                    (inputs.contract_parameters.max_deviation_percent).into(),
                )
                .map_err(RefreshContractError::ErgoTreeConstant)?
                .with_constant(
                    inputs.contract_parameters.epoch_length_index,
                    (inputs.contract_parameters.epoch_length.0).into(),
                )
                .map_err(RefreshContractError::ErgoTreeConstant)?;
        Ok(Self {
            ergo_tree,
            pool_nft_index: inputs.contract_parameters.pool_nft_index,
            oracle_token_id_index: inputs.contract_parameters.oracle_token_id_index,
            min_data_points_index: inputs.contract_parameters.min_data_points_index,
            buffer_index: inputs.contract_parameters.buffer_length_index,
            max_deviation_percent_index: inputs.contract_parameters.max_deviation_percent_index,
            epoch_length_index: inputs.contract_parameters.epoch_length_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn epoch_length(&self) -> EpochLength {
        EpochLength(
            self.ergo_tree
                .get_constant(self.epoch_length_index)
                .unwrap()
                .unwrap()
                .try_extract_into::<i32>()
                .unwrap(),
        )
    }

    pub fn buffer(&self) -> i32 {
        self.ergo_tree
            .get_constant(self.buffer_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap()
    }

    pub fn min_data_points(&self) -> MinDatapoints {
        MinDatapoints(
            self.ergo_tree
                .get_constant(self.min_data_points_index)
                .unwrap()
                .unwrap()
                .try_extract_into::<i32>()
                .unwrap(),
        )
    }

    pub fn max_deviation_percent(&self) -> i32 {
        self.ergo_tree
            .get_constant(self.max_deviation_percent_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap()
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

    pub fn parameters(&self) -> RefreshContractParameters {
        RefreshContractParameters {
            ergo_tree_bytes: self.ergo_tree.sigma_serialize_bytes().unwrap(),
            pool_nft_index: self.pool_nft_index,
            oracle_token_id_index: self.oracle_token_id_index,
            min_data_points_index: self.min_data_points_index,
            min_data_points: self.min_data_points(),
            buffer_length_index: self.buffer_index,
            buffer_length: self.buffer(),
            max_deviation_percent_index: self.max_deviation_percent_index,
            max_deviation_percent: self.max_deviation_percent(),
            epoch_length_index: self.epoch_length_index,
            epoch_length: self.epoch_length(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RefreshContractInputs {
    contract_parameters: RefreshContractParameters,
    pub oracle_token_id: OracleTokenId,
    pub pool_nft_token_id: PoolTokenId,
}

impl RefreshContractInputs {
    pub fn build_with(
        contract_parameters: RefreshContractParameters,
        oracle_token_id: OracleTokenId,
        pool_nft_token_id: PoolTokenId,
    ) -> Result<Self, RefreshContractError> {
        let refresh_contract = RefreshContract::build_with(&RefreshContractInputs {
            contract_parameters,
            oracle_token_id: oracle_token_id.clone(),
            pool_nft_token_id: pool_nft_token_id.clone(),
        })?;
        let new_parameters = refresh_contract.parameters();
        Ok(Self {
            contract_parameters: new_parameters,
            oracle_token_id,
            pool_nft_token_id,
        })
    }

    pub fn checked_load(
        contract_parameters: RefreshContractParameters,
        oracle_token_id: OracleTokenId,
        pool_nft_token_id: PoolTokenId,
    ) -> Result<Self, RefreshContractError> {
        let contract_inputs = RefreshContractInputs {
            contract_parameters,
            oracle_token_id,
            pool_nft_token_id,
        };
        let _refresh_contract = RefreshContract::checked_load(&contract_inputs)?;
        Ok(contract_inputs)
    }

    pub fn contract_parameters(&self) -> &RefreshContractParameters {
        &self.contract_parameters
    }
}

#[derive(Debug, Clone)]
/// Parameters for the pool contract
pub struct RefreshContractParameters {
    ergo_tree_bytes: Vec<u8>,
    pool_nft_index: usize,
    oracle_token_id_index: usize,
    min_data_points_index: usize,
    min_data_points: MinDatapoints,
    buffer_length_index: usize,
    buffer_length: i32,
    max_deviation_percent_index: usize,
    max_deviation_percent: i32,
    epoch_length_index: usize,
    epoch_length: EpochLength,
}

pub struct RefreshContractParametersInputs {
    pub ergo_tree_bytes: Vec<u8>,
    pub pool_nft_index: usize,
    pub oracle_token_id_index: usize,
    pub min_data_points_index: usize,
    pub min_data_points: MinDatapoints,
    pub buffer_length_index: usize,
    pub buffer_length: i32,
    pub max_deviation_percent_index: usize,
    pub max_deviation_percent: i32,
    pub epoch_length_index: usize,
    pub epoch_length: EpochLength,
}

#[derive(Debug, Error)]
pub enum RefreshContractParametersError {
    #[error("refresh contract parameters: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("refresh contract parameters: failed to get oracle token id from constants")]
    NoOracleTokenId,
    #[error("refresh contract parameters: failed to get min data points from constants")]
    NoMinDataPoints,
    #[error(
        "refresh contract parameters: unexpected `min data points` value from constants. Expected {expected}, got {actual}"
    )]
    MinDataPointsDiffers { expected: i32, actual: i32 },
    #[error("refresh contract parameters: failed to get buffer length from constants")]
    NoBufferLength,
    #[error(
        "refresh contract parameters: unexpected `buffer length` value from constants. Expected {expected}, got {actual}"
    )]
    BufferLengthDiffers { expected: i32, actual: i32 },
    #[error("refresh contract parameters: failed to get max deviation percent from constants")]
    NoMaxDeviationPercent,
    #[error(
        "refresh contract parameters: unexpected `max deviation percentage` value from constants. Expected {expected}, got {actual}"
    )]
    MaxDeviationPercentDiffers { expected: i32, actual: i32 },
    #[error("refresh contract parameters: failed to get epoch length from constants")]
    NoEpochLength,
    #[error(
        "refresh contract parameters: unexpected `epoch length` value from constants. Expected {expected:?}, got {actual:?}"
    )]
    EpochLengthDiffers {
        expected: EpochLength,
        actual: EpochLength,
    },
    #[error("refresh contract parameters: sigma parsing error {0}")]
    SigmaParsing(#[from] SigmaParsingError),
    #[error("refresh contract parameters: sigma serialization error {0}")]
    SigmaSerialization(#[from] SigmaSerializationError),
    #[error("refresh contract parameters: base16 decoding error {0}")]
    Decode(#[from] DecodeError),
    #[error("refresh contract parameters: TryExtractFrom error {0:?}")]
    TryExtractFrom(#[from] TryExtractFromError),
    #[error("refresh contract parameters: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
}

impl RefreshContractParameters {
    pub fn build_with(
        inputs: RefreshContractParametersInputs,
    ) -> Result<Self, RefreshContractParametersError> {
        let ergo_tree_orig = ErgoTree::sigma_parse_bytes(inputs.ergo_tree_bytes.as_slice())?;
        log::debug!("refresh contract ergo_tree_orig: {:#?}", ergo_tree_orig);
        let ergo_tree = ergo_tree_orig
            .with_constant(
                inputs.min_data_points_index,
                inputs.min_data_points.0.into(),
            )
            .map_err(RefreshContractParametersError::ErgoTreeConstant)?
            .with_constant(inputs.buffer_length_index, inputs.buffer_length.into())
            .map_err(RefreshContractParametersError::ErgoTreeConstant)?
            .with_constant(
                inputs.max_deviation_percent_index,
                inputs.max_deviation_percent.into(),
            )
            .map_err(RefreshContractParametersError::ErgoTreeConstant)?
            .with_constant(inputs.epoch_length_index, inputs.epoch_length.0.into())
            .map_err(RefreshContractParametersError::ErgoTreeConstant)?;
        let _pool_nft = ergo_tree
            .get_constant(inputs.pool_nft_index)
            .map_err(|_| RefreshContractParametersError::NoPoolNftId)?
            .ok_or(RefreshContractParametersError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        let _oracle_token = ergo_tree
            .get_constant(inputs.oracle_token_id_index)
            .map_err(|_| RefreshContractParametersError::NoOracleTokenId)?
            .ok_or(RefreshContractParametersError::NoOracleTokenId)?
            .try_extract_into::<TokenId>()?;
        Ok(Self {
            ergo_tree_bytes: base16::decode(&ergo_tree.to_base16_bytes()?)?,
            pool_nft_index: inputs.pool_nft_index,
            oracle_token_id_index: inputs.oracle_token_id_index,
            min_data_points_index: inputs.min_data_points_index,
            min_data_points: inputs.min_data_points,
            buffer_length_index: inputs.buffer_length_index,
            buffer_length: inputs.buffer_length,
            max_deviation_percent_index: inputs.max_deviation_percent_index,
            max_deviation_percent: inputs.max_deviation_percent,
            epoch_length_index: inputs.epoch_length_index,
            epoch_length: inputs.epoch_length,
        })
    }

    pub fn checked_load(
        inputs: RefreshContractParametersInputs,
    ) -> Result<Self, RefreshContractParametersError> {
        let ergo_tree = ErgoTree::sigma_parse_bytes(inputs.ergo_tree_bytes.as_slice())?;
        let min_data_points = ergo_tree
            .get_constant(inputs.min_data_points_index)
            .map_err(|_| RefreshContractParametersError::NoMinDataPoints)?
            .ok_or(RefreshContractParametersError::NoMinDataPoints)?
            .try_extract_into::<i32>()?;
        if min_data_points != inputs.min_data_points.0 {
            return Err(RefreshContractParametersError::MinDataPointsDiffers {
                expected: inputs.min_data_points.0,
                actual: min_data_points,
            });
        }

        let buffer_length = ergo_tree
            .get_constant(inputs.buffer_length_index)
            .map_err(|_| RefreshContractParametersError::NoBufferLength)?
            .ok_or(RefreshContractParametersError::NoBufferLength)?
            .try_extract_into::<i32>()?;

        if buffer_length != inputs.buffer_length {
            return Err(RefreshContractParametersError::BufferLengthDiffers {
                expected: inputs.buffer_length,
                actual: buffer_length,
            });
        }

        let max_deviation_percent = ergo_tree
            .get_constant(inputs.max_deviation_percent_index)
            .map_err(|_| RefreshContractParametersError::NoMaxDeviationPercent)?
            .ok_or(RefreshContractParametersError::NoMaxDeviationPercent)?
            .try_extract_into::<i32>()?;

        if max_deviation_percent != inputs.max_deviation_percent {
            return Err(RefreshContractParametersError::MaxDeviationPercentDiffers {
                expected: inputs.max_deviation_percent,
                actual: max_deviation_percent,
            });
        }

        let epoch_length = EpochLength(
            ergo_tree
                .get_constant(inputs.epoch_length_index)
                .map_err(|_| RefreshContractParametersError::NoEpochLength)?
                .ok_or(RefreshContractParametersError::NoEpochLength)?
                .try_extract_into::<i32>()?,
        );

        if epoch_length != inputs.epoch_length {
            return Err(RefreshContractParametersError::EpochLengthDiffers {
                expected: inputs.epoch_length,
                actual: epoch_length,
            });
        }

        let _pool_nft = ergo_tree
            .get_constant(inputs.pool_nft_index)
            .map_err(|_| RefreshContractParametersError::NoPoolNftId)?
            .ok_or(RefreshContractParametersError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        let _oracle_token = ergo_tree
            .get_constant(inputs.oracle_token_id_index)
            .map_err(|_| RefreshContractParametersError::NoOracleTokenId)?
            .ok_or(RefreshContractParametersError::NoOracleTokenId)?
            .try_extract_into::<TokenId>()?;
        Ok(Self {
            ergo_tree_bytes: base16::decode(&ergo_tree.to_base16_bytes()?)?,
            pool_nft_index: inputs.pool_nft_index,
            oracle_token_id_index: inputs.oracle_token_id_index,
            min_data_points_index: inputs.min_data_points_index,
            min_data_points: inputs.min_data_points,
            buffer_length_index: inputs.buffer_length_index,
            buffer_length: inputs.buffer_length,
            max_deviation_percent_index: inputs.max_deviation_percent_index,
            max_deviation_percent: inputs.max_deviation_percent,
            epoch_length_index: inputs.epoch_length_index,
            epoch_length: inputs.epoch_length,
        })
    }

    pub fn ergo_tree_bytes(&self) -> Vec<u8> {
        self.ergo_tree_bytes.clone()
    }

    pub fn pool_nft_index(&self) -> usize {
        self.pool_nft_index
    }

    pub fn oracle_token_id_index(&self) -> usize {
        self.oracle_token_id_index
    }

    pub fn min_data_points_index(&self) -> usize {
        self.min_data_points_index
    }

    pub fn min_data_points(&self) -> MinDatapoints {
        self.min_data_points
    }

    pub fn buffer_length_index(&self) -> usize {
        self.buffer_length_index
    }

    pub fn buffer_length(&self) -> i32 {
        self.buffer_length
    }

    pub fn max_deviation_percent_index(&self) -> usize {
        self.max_deviation_percent_index
    }

    pub fn max_deviation_percent(&self) -> i32 {
        self.max_deviation_percent
    }

    pub fn epoch_length_index(&self) -> usize {
        self.epoch_length_index
    }

    pub fn epoch_length(&self) -> EpochLength {
        self.epoch_length
    }
}

#[cfg(test)]
mod tests {

    use ergo_lib::ergo_chain_types::Digest32;
    use sigma_test_util::force_any_val;

    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = RefreshContractParameters::default();
        let token_ids = generate_token_ids();
        let inputs = RefreshContractInputs {
            contract_parameters: parameters.clone(),
            oracle_token_id: token_ids.oracle_token_id.clone(),
            pool_nft_token_id: token_ids.pool_nft_token_id.clone(),
        };
        let c = RefreshContract::build_with(&inputs).unwrap();
        assert_eq!(
            c.pool_nft_token_id(),
            token_ids.pool_nft_token_id.token_id(),
        );
        assert_eq!(c.oracle_token_id(), token_ids.oracle_token_id.token_id(),);
        assert_eq!(c.min_data_points(), parameters.min_data_points);
        assert_eq!(c.buffer(), parameters.buffer_length);
        assert_eq!(c.max_deviation_percent(), parameters.max_deviation_percent);
        assert_eq!(c.epoch_length(), parameters.epoch_length);
    }

    #[test]
    fn test_build_with() {
        let contract_parameters = RefreshContractParameters::default();
        let expected_min_data_points = MinDatapoints(99);
        let expected_buffer_length = 100;
        let expected_max_deviation_percent = 88;
        let expected_epoch_length = EpochLength(1000);
        let new_contract_parameter_inputs = RefreshContractParametersInputs {
            ergo_tree_bytes: contract_parameters.ergo_tree_bytes(),
            pool_nft_index: contract_parameters.pool_nft_index(),
            oracle_token_id_index: contract_parameters.oracle_token_id_index(),
            min_data_points_index: contract_parameters.min_data_points_index(),
            min_data_points: expected_min_data_points,
            buffer_length_index: contract_parameters.buffer_length_index(),
            buffer_length: expected_buffer_length,
            max_deviation_percent_index: contract_parameters.max_deviation_percent_index(),
            max_deviation_percent: expected_max_deviation_percent,
            epoch_length_index: contract_parameters.epoch_length_index(),
            epoch_length: expected_epoch_length,
        };
        let new_contract_parameters =
            RefreshContractParameters::build_with(new_contract_parameter_inputs).unwrap();
        let inputs = RefreshContractInputs {
            contract_parameters: new_contract_parameters,
            oracle_token_id: OracleTokenId::from_token_id_unchecked(
                force_any_val::<Digest32>().into(),
            ),
            pool_nft_token_id: PoolTokenId::from_token_id_unchecked(
                force_any_val::<Digest32>().into(),
            ),
        };
        let new_contract = RefreshContract::build_with(&inputs).unwrap();
        assert_eq!(new_contract.min_data_points(), expected_min_data_points);
        assert_eq!(new_contract.buffer(), expected_buffer_length);
        assert_eq!(
            new_contract.max_deviation_percent(),
            expected_max_deviation_percent
        );
        assert_eq!(new_contract.epoch_length(), expected_epoch_length);
        assert_eq!(
            new_contract.oracle_token_id(),
            inputs.oracle_token_id.token_id()
        );
        assert_eq!(
            new_contract.pool_nft_token_id(),
            inputs.pool_nft_token_id.token_id()
        );
    }
}
