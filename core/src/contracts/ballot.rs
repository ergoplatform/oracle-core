use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use thiserror::Error;

use crate::oracle_config::TokenIds;

#[derive(Clone)]
pub struct BallotContract {
    ergo_tree: ErgoTree,
    min_storage_rent_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, Error, From)]
pub enum BallotContractError {
    #[error("ballot contract: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("ballot contract: unknown update NFT defined in constant")]
    UnknownUpdateNftId,
    #[error("ballot contract: failed to get minStorageRent from constants")]
    NoMinStorageRent,
    #[error(
        "ballot contract: unexpected `min storage rent` value. Expected {expected:?}, got {actual:?}"
    )]
    MinStorageRentDiffers { expected: u64, actual: u64 },
    #[error("ballot contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("ballot contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("ballot contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl BallotContract {
    pub fn new(
        parameters: &BallotContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, BallotContractError> {
        let ergo_tree = parameters
            .p2s
            .address()
            .script()?
            .with_constant(
                parameters.min_storage_rent_index,
                (parameters.min_storage_rent as i64).into(),
            )?
            .with_constant(
                parameters.update_nft_index,
                token_ids.update_nft_token_id.clone().into(),
            )?;
        let contract = Self::from_ergo_tree(ergo_tree, parameters, token_ids)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &BallotContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, BallotContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let min_storage_rent = ergo_tree
            .get_constant(parameters.min_storage_rent_index)
            .map_err(|_| BallotContractError::NoMinStorageRent)?
            .ok_or(BallotContractError::NoMinStorageRent)?
            .try_extract_into::<i64>()? as u64;
        if min_storage_rent != parameters.min_storage_rent {
            return Err(BallotContractError::MinStorageRentDiffers {
                expected: parameters.min_storage_rent,
                actual: min_storage_rent,
            });
        }

        let token_id = ergo_tree
            .get_constant(parameters.update_nft_index)
            .map_err(|_| BallotContractError::NoUpdateNftId)?
            .ok_or(BallotContractError::NoUpdateNftId)?
            .try_extract_into::<TokenId>()?;
        if token_id != token_ids.update_nft_token_id {
            return Err(BallotContractError::UnknownUpdateNftId);
        }
        Ok(Self {
            ergo_tree,
            min_storage_rent_index: parameters.min_storage_rent_index,
            update_nft_index: parameters.update_nft_index,
        })
    }

    pub fn min_storage_rent(&self) -> u64 {
        self.ergo_tree
            .get_constant(self.min_storage_rent_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }

    pub fn update_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.update_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }
}

#[derive(Debug, Clone)]
/// Parameters for the ballot contract
pub struct BallotContractParameters {
    pub p2s: NetworkAddress,
    pub min_storage_rent_index: usize,
    pub min_storage_rent: u64,
    pub update_nft_index: usize,
}

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::{generate_token_ids, make_ballot_contract_parameters};

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_ballot_contract_parameters();
        let token_ids = generate_token_ids();
        let c = BallotContract::new(&parameters, &token_ids).unwrap();
        assert_eq!(c.update_nft_token_id(), token_ids.update_nft_token_id,);
    }
}