use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use thiserror::Error;

use crate::box_kind::PoolBoxWrapperInputs;
use crate::oracle_config::TokenIds;

#[derive(Clone)]
pub struct PoolContract {
    ergo_tree: ErgoTree,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, From, Error)]
pub enum PoolContractError {
    #[error("pool contract: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("pool contract: failed to get refresh NFT from constants")]
    NoRefreshNftId,
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
}

pub struct PoolContractInputs<'a> {
    pub contract_parameters: &'a PoolContractParameters,
    pub refresh_nft_token_id: &'a TokenId,
    pub update_nft_token_id: &'a TokenId,
}

impl<'a> From<PoolBoxWrapperInputs<'a>> for PoolContractInputs<'a> {
    fn from(b: PoolBoxWrapperInputs<'a>) -> Self {
        PoolContractInputs {
            contract_parameters: b.contract_parameters,
            update_nft_token_id: b.update_nft_token_id,
            refresh_nft_token_id: b.refresh_nft_token_id,
        }
    }
}

impl<'a> From<(&'a PoolContractParameters, &'a TokenIds)> for PoolContractInputs<'a> {
    fn from(t: (&'a PoolContractParameters, &'a TokenIds)) -> Self {
        let contract_parameters = t.0;
        let token_ids = t.1;
        PoolContractInputs {
            contract_parameters,
            refresh_nft_token_id: &token_ids.refresh_nft_token_id,
            update_nft_token_id: &token_ids.update_nft_token_id,
        }
    }
}

impl PoolContract {
    pub fn new(inputs: PoolContractInputs) -> Result<Self, PoolContractError> {
        let ergo_tree = inputs
            .contract_parameters
            .p2s
            .address()
            .script()?
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
        inputs: PoolContractInputs,
    ) -> Result<Self, PoolContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let refresh_nft_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.refresh_nft_index)
            .map_err(|_| PoolContractError::NoRefreshNftId)?
            .ok_or(PoolContractError::NoRefreshNftId)?
            .try_extract_into::<TokenId>()?;
        if refresh_nft_token_id != *inputs.refresh_nft_token_id {
            return Err(PoolContractError::UnknownRefreshNftId);
        }

        let update_nft_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.update_nft_index)
            .map_err(|_| PoolContractError::NoUpdateNftId)?
            .ok_or(PoolContractError::NoUpdateNftId)?
            .try_extract_into::<TokenId>()?;
        if update_nft_token_id != *inputs.update_nft_token_id {
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
}

#[derive(Debug, Clone)]
/// Parameters for the pool contract
pub struct PoolContractParameters {
    pub p2s: NetworkAddress,
    pub refresh_nft_index: usize,
    pub update_nft_index: usize,
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
            contract_parameters: &contract_parameters,
            refresh_nft_token_id: &token_ids.refresh_nft_token_id,
            update_nft_token_id: &token_ids.update_nft_token_id,
        };
        let c = PoolContract::new(inputs).unwrap();
        assert_eq!(c.refresh_nft_token_id(), token_ids.refresh_nft_token_id,);
        assert_eq!(c.update_nft_token_id(), token_ids.update_nft_token_id,);
    }
}
