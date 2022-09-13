use derive_more::From;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct PoolContract {
    ergo_tree: ErgoTree,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, From, Error)]
pub enum PoolContractError {
    #[error("pool contract: parameter error: {0}")]
    ParametersError(PoolContractParametersError),
    #[error("pool contract: unknown refresh NFT in box")]
    UnknownRefreshNftId,
    #[error("pool contract: unknown update NFT in box")]
    UnknownUpdateNftId,
    #[error("pool contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("pool contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("pool contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
    #[error("contract error: {1:?}, expected P2S: {0}")]
    WrappedWithExpectedP2SAddress(String, Box<Self>),
}

#[derive(Clone, Debug)]
pub struct PoolContractInputs {
    contract_parameters: PoolContractParameters,
    pub refresh_nft_token_id: TokenId,
    pub update_nft_token_id: TokenId,
}

impl PoolContractInputs {
    pub fn build_with(
        contract_parameters: PoolContractParameters,
        refresh_nft_token_id: TokenId,
        update_nft_token_id: TokenId,
    ) -> Result<Self, PoolContractError> {
        let contract_inputs = PoolContractInputs {
            contract_parameters,
            refresh_nft_token_id,
            update_nft_token_id,
        };
        let pool_contract = PoolContract::build_with(&contract_inputs)?;
        let new_parameters = pool_contract.parameters();
        Ok(Self {
            contract_parameters: new_parameters,
            ..contract_inputs
        })
    }

    pub fn checked_load(
        contract_parameters: PoolContractParameters,
        refresh_nft_token_id: TokenId,
        update_nft_token_id: TokenId,
    ) -> Result<Self, PoolContractError> {
        let contract_inputs = PoolContractInputs {
            contract_parameters,
            refresh_nft_token_id,
            update_nft_token_id,
        };
        let _ = PoolContract::checked_load(&contract_inputs)?;
        Ok(contract_inputs)
    }

    pub fn contract_parameters(&self) -> &PoolContractParameters {
        &self.contract_parameters
    }
}

impl PoolContract {
    pub fn checked_load(inputs: &PoolContractInputs) -> Result<Self, PoolContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs).map_err(|e| {
            let expected_base16 = Self::build_with(inputs)
                .unwrap()
                .ergo_tree
                .to_base16_bytes()
                .unwrap();
            PoolContractError::WrappedWithExpectedP2SAddress(expected_base16, e.into())
        })?;
        Ok(contract)
    }

    pub fn build_with(inputs: &PoolContractInputs) -> Result<Self, PoolContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?
                .with_constant(
                    inputs.contract_parameters.refresh_nft_index,
                    inputs.refresh_nft_token_id.clone().into(),
                )?
                .with_constant(
                    inputs.contract_parameters.update_nft_index,
                    inputs.update_nft_token_id.clone().into(),
                )?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: &PoolContractInputs,
    ) -> Result<Self, PoolContractError> {
        // dbg!(ergo_tree.get_constants().unwrap());
        let refresh_nft_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.refresh_nft_index)
            .map_err(|_| {
                PoolContractError::ParametersError(PoolContractParametersError::NoRefreshNftId)
            })?
            .ok_or(PoolContractError::ParametersError(
                PoolContractParametersError::NoRefreshNftId,
            ))?
            .try_extract_into::<TokenId>()?;
        if refresh_nft_token_id != inputs.refresh_nft_token_id {
            return Err(PoolContractError::UnknownRefreshNftId);
        }

        let update_nft_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.update_nft_index)
            .map_err(|_| {
                PoolContractError::ParametersError(PoolContractParametersError::NoUpdateNftId)
            })?
            .ok_or(PoolContractError::ParametersError(
                PoolContractParametersError::NoUpdateNftId,
            ))?
            .try_extract_into::<TokenId>()?;
        if update_nft_token_id != inputs.update_nft_token_id {
            return Err(PoolContractError::UnknownUpdateNftId);
        }
        Ok(Self {
            ergo_tree,
            refresh_nft_index: inputs.contract_parameters.refresh_nft_index,
            update_nft_index: inputs.contract_parameters.update_nft_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn refresh_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.refresh_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn update_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.update_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn parameters(&self) -> PoolContractParameters {
        PoolContractParameters {
            ergo_tree_bytes: self.ergo_tree.sigma_serialize_bytes().unwrap(),
            refresh_nft_index: self.refresh_nft_index,
            update_nft_index: self.update_nft_index,
        }
    }
}

#[derive(Debug, Error, From)]
pub enum PoolContractParametersError {
    #[error("pool contract parameters: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("pool contract parameters: failed to get refresh NFT from constants")]
    NoRefreshNftId,
    #[error("pool contract parameters: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("pool contract parameters: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

#[derive(Debug, Clone)]
/// Parameters for the pool contract
pub struct PoolContractParameters {
    ergo_tree_bytes: Vec<u8>,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

impl PoolContractParameters {
    pub fn build_with(
        ergo_tree_bytes: Vec<u8>,
        refresh_nft_index: usize,
        update_nft_index: usize,
    ) -> Result<Self, PoolContractParametersError> {
        let ergo_tree = ErgoTree::sigma_parse_bytes(ergo_tree_bytes.as_slice())?;

        let _refresh_nft = ergo_tree
            .get_constant(refresh_nft_index)
            .map_err(|_| PoolContractParametersError::NoRefreshNftId)?
            .ok_or(PoolContractParametersError::NoRefreshNftId)?
            .try_extract_into::<TokenId>()?;

        let _update_nft = ergo_tree
            .get_constant(refresh_nft_index)
            .map_err(|_| PoolContractParametersError::NoUpdateNftId)?
            .ok_or(PoolContractParametersError::NoUpdateNftId)?
            .try_extract_into::<TokenId>()?;
        Ok(Self {
            ergo_tree_bytes,
            refresh_nft_index,
            update_nft_index,
        })
    }

    pub fn ergo_tree_bytes(&self) -> Vec<u8> {
        self.ergo_tree_bytes.clone()
    }

    pub fn refresh_nft_index(&self) -> usize {
        self.refresh_nft_index
    }

    pub fn update_nft_index(&self) -> usize {
        self.update_nft_index
    }
}

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let contract_parameters = PoolContractParameters::default();
        let token_ids = generate_token_ids();
        let inputs = PoolContractInputs {
            contract_parameters,
            refresh_nft_token_id: token_ids.refresh_nft_token_id.clone(),
            update_nft_token_id: token_ids.update_nft_token_id.clone(),
        };
        let c = PoolContract::build_with(&inputs).unwrap();
        assert_eq!(c.refresh_nft_token_id(), token_ids.refresh_nft_token_id,);
        assert_eq!(c.update_nft_token_id(), token_ids.update_nft_token_id,);
    }
}
