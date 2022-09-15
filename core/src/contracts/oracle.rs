use derive_more::From;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use thiserror::Error;

#[derive(Clone)]
pub struct OracleContract {
    ergo_tree: ErgoTree,
    pool_nft_index: usize,
}

#[derive(Debug, Error)]
pub enum OracleContractError {
    #[error("oracle contract: parameter error: {0}")]
    ParametersError(OracleContractParametersError),
    #[error("oracle contract: expected pool NFT {expected:?}, got {got:?} defined in constant")]
    UnknownPoolNftId { expected: TokenId, got: TokenId },
    #[error("oracle contract: sigma parsing error {0}")]
    SigmaParsing(#[from] SigmaParsingError),
    #[error("oracle contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("oracle contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(#[from] TryExtractFromError),
    #[error("contract error: {1:?}, expected P2S: {0}")]
    WrappedWithExpectedP2SAddress(String, Box<Self>),
}

#[derive(Clone, Debug)]
pub struct OracleContractInputs {
    contract_parameters: OracleContractParameters,
    pub pool_nft_token_id: TokenId,
}

impl OracleContractInputs {
    pub fn build_with(
        contract_parameters: OracleContractParameters,
        pool_nft_token_id: TokenId,
    ) -> Result<Self, OracleContractError> {
        let oracle_contract = OracleContract::build_with(&OracleContractInputs {
            contract_parameters,
            pool_nft_token_id: pool_nft_token_id.clone(),
        })?;
        let new_parameters = oracle_contract.parameters();
        Ok(Self {
            contract_parameters: new_parameters,
            pool_nft_token_id,
        })
    }

    pub fn checked_load(
        contract_parameters: OracleContractParameters,
        pool_nft_token_id: TokenId,
    ) -> Result<Self, OracleContractError> {
        let contract_inputs = OracleContractInputs {
            contract_parameters: contract_parameters.clone(),
            pool_nft_token_id: pool_nft_token_id.clone(),
        };
        let _ = OracleContract::checked_load(&contract_inputs)?;
        Ok(contract_inputs)
    }

    pub fn contract_parameters(&self) -> &OracleContractParameters {
        &self.contract_parameters
    }
}

impl OracleContract {
    pub fn checked_load(inputs: &OracleContractInputs) -> Result<Self, OracleContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs).map_err(|e| {
            let expected_base16 = Self::build_with(inputs)
                .unwrap()
                .ergo_tree
                .to_base16_bytes()
                .unwrap();
            OracleContractError::WrappedWithExpectedP2SAddress(expected_base16, e.into())
        })?;
        Ok(contract)
    }

    fn build_with(inputs: &OracleContractInputs) -> Result<Self, OracleContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?
                .with_constant(
                    inputs.contract_parameters.pool_nft_index,
                    inputs.pool_nft_token_id.clone().into(),
                )
                .map_err(OracleContractError::ErgoTreeConstant)?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs)?;
        Ok(contract)
    }

    /// Create new contract from existing ergo tree, returning error if parameters differ.
    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: &OracleContractInputs,
    ) -> Result<Self, OracleContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        let pool_nft_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.pool_nft_index)
            .map_err(|_| {
                OracleContractError::ParametersError(OracleContractParametersError::NoPoolNftId)
            })?
            .ok_or(OracleContractError::ParametersError(
                OracleContractParametersError::NoPoolNftId,
            ))?
            .try_extract_into::<TokenId>()?;
        if pool_nft_token_id != inputs.pool_nft_token_id {
            return Err(OracleContractError::UnknownPoolNftId {
                expected: inputs.pool_nft_token_id.clone(),
                got: pool_nft_token_id,
            });
        }

        Ok(Self {
            ergo_tree,
            pool_nft_index: inputs.contract_parameters.pool_nft_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn pool_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.pool_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn parameters(&self) -> OracleContractParameters {
        OracleContractParameters {
            ergo_tree_bytes: self.ergo_tree.sigma_serialize_bytes().unwrap(),
            pool_nft_index: self.pool_nft_index,
        }
    }
}

#[derive(Debug, Error, From)]
pub enum OracleContractParametersError {
    #[error("oracle contract parameters: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("ballot contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("oracle contract parameters: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

#[derive(Debug, Clone)]
/// Parameters for the oracle contract
pub struct OracleContractParameters {
    ergo_tree_bytes: Vec<u8>,
    pool_nft_index: usize,
}

impl OracleContractParameters {
    pub fn build_with(
        ergo_tree_bytes: Vec<u8>,
        pool_nft_index: usize,
    ) -> Result<Self, OracleContractParametersError> {
        let ergo_tree = ErgoTree::sigma_parse_bytes(ergo_tree_bytes.as_slice())?;

        let _pool_nft = ergo_tree
            .get_constant(pool_nft_index)
            .map_err(|_| OracleContractParametersError::NoPoolNftId)?
            .ok_or(OracleContractParametersError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        Ok(Self {
            ergo_tree_bytes,
            pool_nft_index,
        })
    }

    pub fn ergo_tree_bytes(&self) -> Vec<u8> {
        self.ergo_tree_bytes.clone()
    }

    pub fn pool_nft_index(&self) -> usize {
        self.pool_nft_index
    }
}

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let contract_parameters = OracleContractParameters::default();
        let token_ids = generate_token_ids();
        let inputs = OracleContractInputs {
            contract_parameters,
            pool_nft_token_id: token_ids.pool_nft_token_id.clone(),
        };
        let c = OracleContract::build_with(&inputs).unwrap();
        assert_eq!(c.pool_nft_token_id(), token_ids.pool_nft_token_id,);
    }
}
