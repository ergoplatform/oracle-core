use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use thiserror::Error;

use crate::contracts::refresh::RefreshContract;
use crate::contracts::refresh::RefreshContractError;
use crate::contracts::refresh::RefreshContractInputs;
use crate::contracts::refresh::RefreshContractParameters;
use crate::spec_token::OracleTokenId;
use crate::spec_token::PoolTokenId;
use crate::spec_token::RefreshTokenId;
use crate::spec_token::TokenIdKind;

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
pub struct RefreshBoxWrapper {
    ergo_box: ErgoBox,
    contract: RefreshContract,
}

#[derive(Clone, Debug)]
pub struct RefreshBoxWrapperInputs {
    pub contract_inputs: RefreshContractInputs,
    /// Refresh token is expected to reside in `tokens(0)` of the oracle box.
    pub refresh_nft_token_id: RefreshTokenId,
}

impl RefreshBoxWrapperInputs {
    pub fn build_with(
        refresh_contract_parameters: RefreshContractParameters,
        oracle_token_id: OracleTokenId,
        pool_token_id: PoolTokenId,
        refresh_nft_token_id: RefreshTokenId,
    ) -> Result<Self, RefreshContractError> {
        let contract_inputs = RefreshContractInputs::build_with(
            refresh_contract_parameters,
            oracle_token_id,
            pool_token_id,
        )?;
        Ok(RefreshBoxWrapperInputs {
            contract_inputs,
            refresh_nft_token_id,
        })
    }

    pub fn checked_load(
        refresh_contract_parameters: RefreshContractParameters,
        oracle_token_id: OracleTokenId,
        pool_token_id: PoolTokenId,
        refresh_nft_token_id: RefreshTokenId,
    ) -> Result<Self, RefreshContractError> {
        let contract_inputs = RefreshContractInputs::checked_load(
            refresh_contract_parameters,
            oracle_token_id,
            pool_token_id,
        )?;
        Ok(RefreshBoxWrapperInputs {
            contract_inputs,
            refresh_nft_token_id,
        })
    }
}

impl RefreshBoxWrapper {
    pub fn new(b: ErgoBox, inputs: &RefreshBoxWrapperInputs) -> Result<Self, RefreshBoxError> {
        let refresh_token_id = b
            .tokens
            .as_ref()
            .ok_or(RefreshBoxError::NoTokens)?
            .get(0)
            .ok_or(RefreshBoxError::NoTokens)?
            .token_id
            .clone();
        if refresh_token_id != inputs.refresh_nft_token_id.token_id() {
            return Err(RefreshBoxError::IncorrectRefreshTokenId(refresh_token_id));
        }

        let contract =
            RefreshContract::from_ergo_tree(b.ergo_tree.clone(), &inputs.contract_inputs)?;
        Ok(Self {
            ergo_box: b,
            contract,
        })
    }
}

impl RefreshBox for RefreshBoxWrapper {
    fn refresh_nft_token(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    fn contract(&self) -> &RefreshContract {
        &self.contract
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
