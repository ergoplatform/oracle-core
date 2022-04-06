use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use thiserror::Error;

use crate::contracts::refresh::RefreshContract;

pub trait PoolBox {
    fn pool_nft_token(&self) -> Token;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> u64;
    fn get_box(&self) -> ErgoBox;
}

#[derive(Debug, Error)]
pub enum PoolBoxError {
    #[error("pool box: incorrect pool token id: {0:?}")]
    IncorrectPoolTokenId(TokenId),
    #[error("pool box: no tokens found")]
    NoTokens,
    #[error("pool box: no data point in R4")]
    NoDataPoint,
    #[error("pool box: no epoch counter in R5")]
    NoEpochCounter,
}

#[derive(Clone)]
pub struct PoolBoxWrapper(ErgoBox);

impl PoolBox for PoolBoxWrapper {
    fn pool_nft_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(0).unwrap().clone()
    }

    fn epoch_counter(&self) -> u32 {
        self.0
            .get_register(NonMandatoryRegisterId::R5.into())
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    fn rate(&self) -> u64 {
        self.0
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }

    fn get_box(&self) -> ErgoBox {
        self.0.clone()
    }
}

impl TryFrom<ErgoBox> for PoolBoxWrapper {
    type Error = PoolBoxError;

    fn try_from(b: ErgoBox) -> Result<Self, Self::Error> {
        let refresh_contract = RefreshContract::new();
        let pool_token_id = b
            .tokens
            .as_ref()
            .ok_or(PoolBoxError::NoTokens)?
            .get(0)
            .ok_or(PoolBoxError::NoTokens)?
            .token_id
            .clone();
        if pool_token_id != refresh_contract.pool_nft_token_id() {
            return Err(PoolBoxError::IncorrectPoolTokenId(pool_token_id));
        }

        if b.get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(PoolBoxError::NoDataPoint)?
            .try_extract_into::<i64>()
            .is_err()
        {
            return Err(PoolBoxError::NoDataPoint);
        }

        if b.get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(PoolBoxError::NoEpochCounter)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(PoolBoxError::NoEpochCounter);
        }

        Ok(Self(b))
    }
}
