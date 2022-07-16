use std::convert::TryFrom;

use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::{Literal, TryExtractInto};
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::oracle_config::TokenIds;

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
    #[error("oracle contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("oracle contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("oracle contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl UpdateContract {
    pub fn new(
        parameters: &UpdateContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, UpdateContractError> {
        let ergo_tree = parameters
            .p2s
            .address()
            .script()?
            .with_constant(
                parameters.pool_nft_index,
                token_ids.pool_nft_token_id.clone().into(),
            )?
            .with_constant(
                parameters.ballot_token_index,
                token_ids.ballot_token_id.clone().into(),
            )?;
        let contract = Self::from_ergo_tree(ergo_tree, parameters, token_ids)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &UpdateContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, UpdateContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let pool_nft_token_id = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| UpdateContractError::NoPoolNftId)?
            .ok_or(UpdateContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>()?;
        if pool_nft_token_id != token_ids.pool_nft_token_id {
            return Err(UpdateContractError::UnknownPoolNftId);
        };

        let ballot_token_id = ergo_tree
            .get_constant(parameters.ballot_token_index)
            .map_err(|_| UpdateContractError::NoBallotTokenId)?
            .ok_or(UpdateContractError::NoBallotTokenId)?
            .try_extract_into::<TokenId>()?;
        if ballot_token_id != token_ids.ballot_token_id {
            return Err(UpdateContractError::UnknownBallotTokenId);
        };

        let min_votes = ergo_tree
            .get_constant(parameters.min_votes_index)
            .map_err(|_| UpdateContractError::NoMinVotes)?
            .ok_or(UpdateContractError::NoMinVotes)?
            .try_extract_into::<i32>()? as u64;
        if min_votes != parameters.min_votes {
            return Err(UpdateContractError::MinVotesDiffers {
                expected: parameters.min_votes,
                actual: min_votes,
            });
        };
        Ok(Self {
            ergo_tree,
            pool_nft_index: parameters.pool_nft_index,
            ballot_token_index: parameters.ballot_token_index,
            min_votes_index: parameters.min_votes_index,
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    try_from = "UpdateContractParametersYaml",
    into = "UpdateContractParametersYaml"
)]
/// Parameters for the update contract
pub struct UpdateContractParameters {
    pub p2s: NetworkAddress,
    pub pool_nft_index: usize,
    pub ballot_token_index: usize,
    pub min_votes_index: usize,
    pub min_votes: u64,
}

/// Used to (de)serialize `OracleContractParameters` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateContractParametersYaml {
    p2s: String,
    on_mainnet: bool,
    pool_nft_index: usize,
    ballot_token_index: usize,
    min_votes_index: usize,
    min_votes: u64,
}

impl TryFrom<UpdateContractParametersYaml> for UpdateContractParameters {
    type Error = AddressEncoderError;

    fn try_from(p: UpdateContractParametersYaml) -> Result<Self, Self::Error> {
        let prefix = if p.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        let address = AddressEncoder::new(prefix).parse_address_from_str(&p.p2s)?;
        Ok(UpdateContractParameters {
            p2s: NetworkAddress::new(prefix, &address),
            pool_nft_index: p.pool_nft_index,
            ballot_token_index: p.ballot_token_index,
            min_votes_index: p.min_votes_index,
            min_votes: p.min_votes,
        })
    }
}

impl From<UpdateContractParameters> for UpdateContractParametersYaml {
    fn from(p: UpdateContractParameters) -> Self {
        UpdateContractParametersYaml {
            p2s: p.p2s.to_base58(),
            on_mainnet: p.p2s.network() == NetworkPrefix::Mainnet,
            pool_nft_index: p.pool_nft_index,
            ballot_token_index: p.ballot_token_index,
            min_votes_index: p.min_votes_index,
            min_votes: p.min_votes,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::{generate_token_ids, make_update_contract_parameters};

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_update_contract_parameters();
        let token_ids = generate_token_ids();
        let c = UpdateContract::new(&parameters, &token_ids).unwrap();
        assert_eq!(c.pool_nft_token_id(), token_ids.pool_nft_token_id,);
        assert_eq!(c.ballot_token_id(), token_ids.ballot_token_id,);
        assert_eq!(c.min_votes(), parameters.min_votes);
    }
}
