use std::convert::TryFrom;

use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use thiserror::Error;

use crate::contracts::oracle::OracleContractParameters;
use crate::contracts::pool::PoolContract;
use crate::contracts::pool::PoolContractError;
use crate::contracts::pool::PoolContractParameters;
use crate::oracle_config::TokenIds;

pub trait PoolBox {
    fn contract(&self) -> &PoolContract;
    fn pool_nft_token(&self) -> Token;
    fn reward_token(&self) -> Token;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> u64;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Debug, Error)]
pub enum PoolBoxError {
    #[error("pool box: no tokens found")]
    NoTokens,
    #[error("pool box: no data point in R4")]
    NoDataPoint,
    #[error("pool box: no epoch counter in R5")]
    NoEpochCounter,
    #[error("pool box: no reward token found")]
    NoRewardToken,
    #[error("pool box: {0:?}")]
    PoolContractError(#[from] PoolContractError),
    #[error("pool box: unknown pool NFT token id in box")]
    UnknownPoolNftId,
    #[error("pool box: unknown reward token id in box")]
    UnknownRewardTokenId,
}

#[derive(Clone)]
pub struct PoolBoxWrapper(ErgoBox, PoolContract);

impl PoolBoxWrapper {
    pub fn new(
        b: ErgoBox,
        pool_contract_parameters: &PoolContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, PoolBoxError> {
        if let Some(token) = b.tokens.as_ref().ok_or(PoolBoxError::NoTokens)?.get(0) {
            if token.token_id != token_ids.pool_nft_token_id {
                return Err(PoolBoxError::UnknownPoolNftId);
            }
        } else {
            return Err(PoolBoxError::NoTokens);
        }

        // No need to analyse the data point as its validity is checked within the refresh contract.
        if b.get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(PoolBoxError::NoDataPoint)?
            .try_extract_into::<i64>()
            .is_err()
        {
            return Err(PoolBoxError::NoDataPoint);
        }

        // No need to analyse the epoch counter as its validity is checked within the pool and
        // oracle contracts.
        if b.get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(PoolBoxError::NoEpochCounter)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(PoolBoxError::NoEpochCounter);
        }

        if let Some(reward_token) = b.tokens.as_ref().ok_or(PoolBoxError::NoTokens)?.get(1) {
            if reward_token.token_id != token_ids.reward_token_id {
                return Err(PoolBoxError::UnknownRewardTokenId);
            }
        } else {
            return Err(PoolBoxError::NoRewardToken);
        }
        let contract =
            PoolContract::from_ergo_tree(b.ergo_tree.clone(), pool_contract_parameters, token_ids)?;
        Ok(Self(b, contract))
    }
}

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

    fn reward_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(1).unwrap().clone()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.0
    }

    fn contract(&self) -> &PoolContract {
        &self.1
    }
}

impl<'a>
    TryFrom<(
        ErgoBox,
        &'a PoolContractParameters,
        &'a OracleContractParameters,
        &'a TokenIds,
    )> for PoolBoxWrapper
{
    type Error = PoolBoxError;

    fn try_from(
        value: (
            ErgoBox,
            &PoolContractParameters,
            &OracleContractParameters,
            &TokenIds,
        ),
    ) -> Result<Self, Self::Error> {
        PoolBoxWrapper::new(value.0, value.1, value.3)
    }
}

pub fn make_pool_box_candidate(
    contract: &PoolContract,
    datapoint: i64,
    epoch_counter: i32,
    pool_nft_token: Token,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, datapoint.into());
    builder.set_register_value(NonMandatoryRegisterId::R5, epoch_counter.into());
    builder.add_token(pool_nft_token.clone());
    builder.add_token(reward_token.clone());
    builder.build()
}
