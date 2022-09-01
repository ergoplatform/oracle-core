use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
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
    MinDataPointsDiffers { expected: i32, actual: i32 },
    #[error(
        "refresh contract: unexpected `buffer length` value from constants. Expected {expected}, got {actual}"
    )]
    BufferLengthDiffers { expected: i32, actual: i32 },
    #[error(
        "refresh contract: unexpected `max deviation percentage` value from constants. Expected {expected}, got {actual}"
    )]
    MaxDeviationPercentDiffers { expected: i32, actual: i32 },
    #[error(
        "refresh contract: unexpected `epoch length` value from constants. Expected {expected}, got {actual}"
    )]
    EpochLengthDiffers { expected: i32, actual: i32 },
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
    pub fn load(inputs: &RefreshContractInputs) -> Result<Self, RefreshContractError> {
        let ergo_tree = inputs.contract_parameters.p2s.address().script()?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: &RefreshContractInputs,
    ) -> Result<Self, RefreshContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        let parameters = inputs.contract_parameters.clone();
        let pool_nft_token_id = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| RefreshContractError::NoPoolNftId)?
            .ok_or(RefreshContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        if pool_nft_token_id != inputs.pool_nft_token_id {
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
        if oracle_token_id != inputs.oracle_token_id {
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
        if min_data_points != parameters.min_data_points {
            return Err(RefreshContractError::MinDataPointsDiffers {
                expected: parameters.min_data_points,
                actual: min_data_points,
            });
        }

        let buffer_length = ergo_tree
            .get_constant(parameters.buffer_index)
            .map_err(|_| RefreshContractError::NoBuffer)?
            .ok_or(RefreshContractError::NoBuffer)?
            .try_extract_into::<i32>()?;
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
            .try_extract_into::<i32>()?;
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
            .try_extract_into::<i32>()?;
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

    fn create(inputs: RefreshContractInputs) -> Result<Self, RefreshContractError> {
        let ergo_tree = inputs
            .contract_parameters
            .p2s
            .address()
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
            .map_err(RefreshContractError::ErgoTreeConstant)?
            .with_constant(
                inputs.contract_parameters.min_data_points_index,
                (inputs.contract_parameters.min_data_points).into(),
            )
            .map_err(RefreshContractError::ErgoTreeConstant)?
            .with_constant(
                inputs.contract_parameters.buffer_index,
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
                (inputs.contract_parameters.epoch_length).into(),
            )
            .map_err(RefreshContractError::ErgoTreeConstant)?;
        Ok(Self {
            ergo_tree,
            pool_nft_index: inputs.contract_parameters.pool_nft_index,
            oracle_token_id_index: inputs.contract_parameters.oracle_token_id_index,
            min_data_points_index: inputs.contract_parameters.min_data_points_index,
            buffer_index: inputs.contract_parameters.buffer_index,
            max_deviation_percent_index: inputs.contract_parameters.max_deviation_percent_index,
            epoch_length_index: inputs.contract_parameters.epoch_length_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn epoch_length(&self) -> i32 {
        self.ergo_tree
            .get_constant(self.epoch_length_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap()
    }

    pub fn buffer(&self) -> i32 {
        self.ergo_tree
            .get_constant(self.buffer_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap()
    }

    pub fn min_data_points(&self) -> i32 {
        self.ergo_tree
            .get_constant(self.min_data_points_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap()
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

    pub fn parameters(&self, network_prefix: NetworkPrefix) -> RefreshContractParameters {
        RefreshContractParameters {
            p2s: NetworkAddress::new(
                network_prefix,
                &Address::P2S(self.ergo_tree.sigma_serialize_bytes().unwrap()),
            ),
            pool_nft_index: self.pool_nft_index,
            oracle_token_id_index: self.oracle_token_id_index,
            min_data_points_index: self.min_data_points_index,
            min_data_points: self.min_data_points(),
            buffer_index: self.buffer_index,
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
    pub oracle_token_id: TokenId,
    pub pool_nft_token_id: TokenId,
}

impl RefreshContractInputs {
    pub fn new(
        contract_parameters: RefreshContractParameters,
        oracle_token_id: TokenId,
        pool_nft_token_id: TokenId,
    ) -> Result<Self, RefreshContractError> {
        let network_prefix = contract_parameters.p2s.network();
        let refresh_contract = RefreshContract::create(RefreshContractInputs {
            contract_parameters,
            oracle_token_id: oracle_token_id.clone(),
            pool_nft_token_id: pool_nft_token_id.clone(),
        })?;
        let new_parameters = refresh_contract.parameters(network_prefix);
        Ok(Self {
            contract_parameters: new_parameters,
            oracle_token_id,
            pool_nft_token_id,
        })
    }
}

#[derive(Debug, Clone)]
/// Parameters for the pool contract
pub struct RefreshContractParameters {
    pub p2s: NetworkAddress,
    pub pool_nft_index: usize,
    pub oracle_token_id_index: usize,
    pub min_data_points_index: usize,
    pub min_data_points: i32,
    pub buffer_index: usize,
    pub buffer_length: i32,
    pub max_deviation_percent_index: usize,
    pub max_deviation_percent: i32,
    pub epoch_length_index: usize,
    pub epoch_length: i32,
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
            contract_parameters: parameters.clone(),
            oracle_token_id: token_ids.oracle_token_id.clone(),
            pool_nft_token_id: token_ids.pool_nft_token_id.clone(),
        };
        let c = RefreshContract::create(inputs).unwrap();
        assert_eq!(c.pool_nft_token_id(), token_ids.pool_nft_token_id,);
        assert_eq!(c.oracle_token_id(), token_ids.oracle_token_id,);
        assert_eq!(c.min_data_points(), parameters.min_data_points);
        assert_eq!(c.buffer(), parameters.buffer_length);
        assert_eq!(c.max_deviation_percent(), parameters.max_deviation_percent);
        assert_eq!(c.epoch_length(), parameters.epoch_length);
    }
}
