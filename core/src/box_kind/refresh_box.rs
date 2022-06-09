use std::convert::TryFrom;

use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use thiserror::Error;

use crate::contracts::pool::PoolContract;
use crate::contracts::refresh::RefreshContract;
use crate::contracts::refresh::RefreshContractError;

pub trait RefreshBox {
    fn contract(&self) -> &RefreshContract;
    fn refresh_nft_token(&self) -> Token;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Debug, Error)]
pub enum RefreshBoxError {
    #[error("refresh box: no tokens found")]
    NoTokens,
    #[error("refresh box: incorrect refresh token id: {0:?}")]
    IncorrectRefreshTokenId(TokenId),
    #[error("refresh box: incorrect reward token id: {0:?}")]
    IncorrectRewardTokenId(TokenId),
    #[error("refresh box: no reward token found")]
    NoRewardToken,
    #[error("refresh box: refresh contract error: {0:?}")]
    RefreshContractError(#[from] RefreshContractError),
}

#[derive(Clone)]
pub struct RefreshBoxWrapper(ErgoBox, RefreshContract);

impl RefreshBox for RefreshBoxWrapper {
    fn refresh_nft_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(0).unwrap().clone()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.0
    }

    fn contract(&self) -> &RefreshContract {
        &self.1
    }
}

impl TryFrom<ErgoBox> for RefreshBoxWrapper {
    type Error = RefreshBoxError;

    fn try_from(b: ErgoBox) -> Result<Self, Self::Error> {
        let pool_contract = PoolContract::new();
        let refresh_token_id = b
            .tokens
            .as_ref()
            .ok_or(RefreshBoxError::NoTokens)?
            .get(0)
            .ok_or(RefreshBoxError::NoTokens)?
            .token_id
            .clone();
        if refresh_token_id != pool_contract.refresh_nft_token_id() {
            return Err(RefreshBoxError::IncorrectRefreshTokenId(refresh_token_id));
        }

        let contract = RefreshContract::from_ergo_tree(b.ergo_tree.clone())?;
        Ok(Self(b, contract))
    }
}

pub fn make_refresh_box_candidate(
    contract: &RefreshContract,
    refresh_nft: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.add_token(refresh_nft.clone());
    builder.build()
}
