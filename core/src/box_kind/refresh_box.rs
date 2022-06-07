use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use thiserror::Error;

use crate::contracts::pool::PoolContract;

pub trait RefreshBox {
    fn refresh_nft_token(&self) -> Token;
    fn get_box(&self) -> ErgoBox;
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
}

#[derive(Clone)]
pub struct RefreshBoxWrapper(ErgoBox);

impl RefreshBox for RefreshBoxWrapper {
    fn refresh_nft_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(0).unwrap().clone()
    }

    fn get_box(&self) -> ErgoBox {
        self.0.clone()
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

        Ok(Self(b))
    }
}
