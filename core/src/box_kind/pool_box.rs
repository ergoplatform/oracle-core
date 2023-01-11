use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use thiserror::Error;

use crate::contracts::pool::PoolContract;
use crate::contracts::pool::PoolContractError;
use crate::contracts::pool::PoolContractInputs;
use crate::contracts::pool::PoolContractParameters;
use crate::oracle_types::BlockHeight;
use crate::spec_token::PoolTokenId;
use crate::spec_token::RefreshTokenId;
use crate::spec_token::RewardTokenId;
use crate::spec_token::SpecToken;
use crate::spec_token::TokenIdKind;
use crate::spec_token::UpdateTokenId;

pub trait PoolBox {
    fn contract(&self) -> &PoolContract;
    fn pool_nft_token(&self) -> SpecToken<PoolTokenId>;
    fn reward_token(&self) -> SpecToken<RewardTokenId>;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> i64;
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

#[derive(Clone, Debug)]
pub struct PoolBoxWrapper {
    ergo_box: ErgoBox,
    contract: PoolContract,
}

impl PoolBoxWrapper {
    pub fn new(b: ErgoBox, inputs: &PoolBoxWrapperInputs) -> Result<Self, PoolBoxError> {
        if let Some(token) = b.tokens.as_ref().ok_or(PoolBoxError::NoTokens)?.get(0) {
            if token.token_id != inputs.pool_nft_token_id.token_id() {
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
            if reward_token.token_id != inputs.reward_token_id.token_id() {
                return Err(PoolBoxError::UnknownRewardTokenId);
            }
        } else {
            return Err(PoolBoxError::NoRewardToken);
        }
        let contract = PoolContract::from_ergo_tree(b.ergo_tree.clone(), &inputs.contract_inputs)?;
        Ok(Self {
            ergo_box: b,
            contract,
        })
    }
}

impl PoolBox for PoolBoxWrapper {
    fn pool_nft_token(&self) -> SpecToken<PoolTokenId> {
        let token = self
            .ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone();
        // unchecked is safe here as PoolBoxWrapper::new validates token id
        SpecToken {
            token_id: PoolTokenId::from_token_id_unchecked(token.token_id),
            amount: token.amount,
        }
    }

    fn epoch_counter(&self) -> u32 {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R5.into())
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    fn rate(&self) -> i64 {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap()
    }

    fn reward_token(&self) -> SpecToken<RewardTokenId> {
        let token = self
            .ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(1)
            .unwrap()
            .clone();
        SpecToken {
            token_id: RewardTokenId::from_token_id_unchecked(token.token_id),
            amount: token.amount,
        }
    }

    fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    fn contract(&self) -> &PoolContract {
        &self.contract
    }
}

#[derive(Clone, Debug)]
pub struct PoolBoxWrapperInputs {
    pub contract_inputs: PoolContractInputs,
    /// Pool NFT token is expected to reside in `tokens(0)` of the pool box.
    pub pool_nft_token_id: PoolTokenId,
    /// Reward token is expected to reside in `tokens(1)` of the pool box.
    pub reward_token_id: RewardTokenId,
}

impl PoolBoxWrapperInputs {
    pub fn build_with(
        contract_parameters: PoolContractParameters,
        refresh_nft_token_id: RefreshTokenId,
        update_nft_token_id: UpdateTokenId,
        pool_nft_token_id: PoolTokenId,
        reward_token_id: RewardTokenId,
    ) -> Result<Self, PoolContractError> {
        let contract_inputs = PoolContractInputs::build_with(
            contract_parameters,
            refresh_nft_token_id,
            update_nft_token_id,
        )?;
        Ok(Self {
            contract_inputs,
            pool_nft_token_id,
            reward_token_id,
        })
    }

    pub fn checked_load(
        contract_parameters: PoolContractParameters,
        refresh_nft_token_id: RefreshTokenId,
        update_nft_token_id: UpdateTokenId,
        pool_nft_token_id: PoolTokenId,
        reward_token_id: RewardTokenId,
    ) -> Result<Self, PoolContractError> {
        let contract_inputs = PoolContractInputs::checked_load(
            contract_parameters,
            refresh_nft_token_id,
            update_nft_token_id,
        )?;
        Ok(Self {
            contract_inputs,
            pool_nft_token_id,
            reward_token_id,
        })
    }
}

pub fn make_pool_box_candidate(
    contract: &PoolContract,
    datapoint: i64,
    epoch_counter: i32,
    pool_nft_token: SpecToken<PoolTokenId>,
    reward_token: SpecToken<RewardTokenId>,
    value: BoxValue,
    creation_height: BlockHeight,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height.0);
    builder.set_register_value(NonMandatoryRegisterId::R4, datapoint.into());
    builder.set_register_value(NonMandatoryRegisterId::R5, epoch_counter.into());
    builder.add_token(pool_nft_token.into());
    builder.add_token(reward_token.into());
    builder.build()
}

/// Make a pool box without type-checking reward token. Mainly used when updating the pool
pub fn make_pool_box_candidate_unchecked(
    contract: &PoolContract,
    datapoint: i64,
    epoch_counter: i32,
    pool_nft_token: SpecToken<PoolTokenId>,
    reward_token: Token,
    value: BoxValue,
    creation_height: BlockHeight,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height.0);
    builder.set_register_value(NonMandatoryRegisterId::R4, datapoint.into());
    builder.set_register_value(NonMandatoryRegisterId::R5, epoch_counter.into());
    builder.add_token(pool_nft_token.into());
    builder.add_token(reward_token);
    builder.build()
}
