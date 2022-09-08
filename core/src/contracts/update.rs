use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::{Literal, TryExtractInto};
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;

use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use thiserror::Error;

#[derive(Clone)]
pub struct UpdateContract {
    ergo_tree: ErgoTree,
    pool_nft_index: usize,
    ballot_token_index: usize,
    min_votes_index: usize,
}

#[derive(Debug, Error, From)]
pub enum UpdateContractError {
    #[error("update contract: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("update contract: unknown pool NFT defined in constants")]
    UnknownPoolNftId,
    #[error("update contract: failed to get ballot token id from constants")]
    NoBallotTokenId,
    #[error("update contract: unknown ballot token id defined in constants")]
    UnknownBallotTokenId,
    #[error("update contract: failed to get minimum votes (must be SInt)")]
    NoMinVotes,
    #[error(
        "update contract: unexpected `min votes` value from constants. Expected {expected}, got {actual}"
    )]
    MinVotesDiffers { expected: u64, actual: u64 },
    #[error("update contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("update contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("update contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
    #[error("contract error: {1:?}, expected P2S: {0}")]
    WrappedWithExpectedP2SAddress(String, Box<Self>),
}

#[derive(Debug, Clone)]
pub struct UpdateContractInputs {
    contract_parameters: UpdateContractParameters,
    pub pool_nft_token_id: TokenId,
    pub ballot_token_id: TokenId,
}

impl UpdateContractInputs {
    pub fn build_with(
        contract_parameters: UpdateContractParameters,
        pool_nft_token_id: TokenId,
        ballot_token_id: TokenId,
    ) -> Result<Self, UpdateContractError> {
        let network_prefix = contract_parameters.p2s.network();
        let inputs_to_create_contract = Self {
            contract_parameters,
            pool_nft_token_id,
            ballot_token_id,
        };
        let update_contract = UpdateContract::build_with(&inputs_to_create_contract)?;
        let new_parameters = update_contract.parameters(network_prefix);
        Ok(Self {
            contract_parameters: new_parameters,
            ..inputs_to_create_contract
        })
    }

    pub fn checked_load(
        contract_parameters: UpdateContractParameters,
        pool_nft_token_id: TokenId,
        ballot_token_id: TokenId,
    ) -> Result<Self, UpdateContractError> {
        let contract_inputs = Self {
            contract_parameters,
            pool_nft_token_id,
            ballot_token_id,
        };
        let _ = UpdateContract::checked_load(&contract_inputs)?;
        Ok(contract_inputs)
    }

    pub fn contract_parameters(&self) -> &UpdateContractParameters {
        &self.contract_parameters
    }
}

impl UpdateContract {
    fn build_with(inputs: &UpdateContractInputs) -> Result<Self, UpdateContractError> {
        let ergo_tree = inputs
            .contract_parameters
            .p2s
            .address()
            .script()?
            .with_constant(
                inputs.contract_parameters.pool_nft_index,
                inputs.pool_nft_token_id.clone().into(),
            )?
            .with_constant(
                inputs.contract_parameters.ballot_token_index,
                inputs.ballot_token_id.clone().into(),
            )?
            .with_constant(
                inputs.contract_parameters.min_votes_index,
                (inputs.contract_parameters.min_votes as i32).into(),
            )?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs)?;
        Ok(contract)
    }

    pub fn checked_load(inputs: &UpdateContractInputs) -> Result<Self, UpdateContractError> {
        let ergo_tree = inputs.contract_parameters.p2s.address().script()?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs).map_err(|e| {
            let expected_p2s = NetworkAddress::new(
                inputs.contract_parameters().p2s.network(),
                &Address::P2S(
                    Self::build_with(inputs)
                        .unwrap()
                        .ergo_tree
                        .sigma_serialize_bytes()
                        .unwrap(),
                ),
            )
            .to_base58();
            UpdateContractError::WrappedWithExpectedP2SAddress(expected_p2s, e.into())
        })?;
        Ok(contract)
    }
    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: &UpdateContractInputs,
    ) -> Result<Self, UpdateContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let pool_nft_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.pool_nft_index)
            .map_err(|_| UpdateContractError::NoPoolNftId)?
            .ok_or(UpdateContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        if pool_nft_token_id != inputs.pool_nft_token_id {
            return Err(UpdateContractError::UnknownPoolNftId);
        };

        let ballot_token_id = ergo_tree
            .get_constant(inputs.contract_parameters.ballot_token_index)
            .map_err(|_| UpdateContractError::NoBallotTokenId)?
            .ok_or(UpdateContractError::NoBallotTokenId)?
            .try_extract_into::<TokenId>()?;
        if ballot_token_id != inputs.ballot_token_id {
            return Err(UpdateContractError::UnknownBallotTokenId);
        };

        let min_votes = ergo_tree
            .get_constant(inputs.contract_parameters.min_votes_index)
            .map_err(|_| UpdateContractError::NoMinVotes)?
            .ok_or(UpdateContractError::NoMinVotes)?
            .try_extract_into::<i32>()? as u64;
        if min_votes != inputs.contract_parameters.min_votes {
            return Err(UpdateContractError::MinVotesDiffers {
                expected: inputs.contract_parameters.min_votes,
                actual: min_votes,
            });
        };
        Ok(Self {
            ergo_tree,
            pool_nft_index: inputs.contract_parameters.pool_nft_index,
            ballot_token_index: inputs.contract_parameters.ballot_token_index,
            min_votes_index: inputs.contract_parameters.min_votes_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn min_votes(&self) -> u64 {
        let vote_constant = self
            .ergo_tree
            .get_constant(self.min_votes_index)
            .unwrap()
            .unwrap();
        if let Literal::Int(votes) = vote_constant.v {
            votes as u64
        } else {
            panic!(
                "update: minimum votes is wrong type, expected SInt, found {:?}",
                vote_constant.tpe
            );
        }
    }

    pub fn pool_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.pool_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn ballot_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.ballot_token_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn parameters(&self, network_prefix: NetworkPrefix) -> UpdateContractParameters {
        UpdateContractParameters {
            p2s: NetworkAddress::new(
                network_prefix,
                &Address::P2S(self.ergo_tree.sigma_serialize_bytes().unwrap()),
            ),
            pool_nft_index: self.pool_nft_index,
            ballot_token_index: self.ballot_token_index,
            min_votes_index: self.min_votes_index,
            min_votes: self.min_votes(),
        }
    }
}

#[derive(Debug, Clone)]
/// Parameters for the update contract
pub struct UpdateContractParameters {
    pub p2s: NetworkAddress,
    pub pool_nft_index: usize,
    pub ballot_token_index: usize,
    pub min_votes_index: usize,
    pub min_votes: u64,
}

#[cfg(test)]
mod tests {

    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = UpdateContractParameters::default();
        let token_ids = generate_token_ids();
        let inputs = UpdateContractInputs {
            contract_parameters: parameters.clone(),
            pool_nft_token_id: token_ids.pool_nft_token_id.clone(),
            ballot_token_id: token_ids.ballot_token_id.clone(),
        };
        let c = UpdateContract::build_with(&inputs).unwrap();
        assert_eq!(c.pool_nft_token_id(), token_ids.pool_nft_token_id,);
        assert_eq!(c.ballot_token_id(), token_ids.ballot_token_id,);
        assert_eq!(c.min_votes(), parameters.min_votes);
    }
}
