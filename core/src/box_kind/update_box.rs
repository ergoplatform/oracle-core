use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use thiserror::Error;

use crate::contracts::update::UpdateContract;
use crate::contracts::update::UpdateContractError;
use crate::contracts::update::UpdateContractInputs;
use crate::contracts::update::UpdateContractParameters;
use crate::spec_token::BallotTokenId;
use crate::spec_token::PoolTokenId;
use crate::spec_token::TokenIdKind;
use crate::spec_token::UpdateTokenId;

#[derive(Debug, Error)]
pub enum UpdateBoxError {
    #[error("oracle box: no tokens found")]
    NoTokens,
    #[error("update contract: {0:?}")]
    UpdateContractError(#[from] UpdateContractError),
    #[error("update contract: {0:?}")]
    IncorrectUpdateTokenId(TokenId),
}

#[derive(Clone)]
pub struct UpdateBoxWrapper {
    ergo_box: ErgoBox,
    contract: UpdateContract,
}

impl UpdateBoxWrapper {
    pub fn new(b: ErgoBox, inputs: &UpdateBoxWrapperInputs) -> Result<Self, UpdateBoxError> {
        let update_token_id = b
            .tokens
            .as_ref()
            .ok_or(UpdateBoxError::NoTokens)?
            .get(0)
            .ok_or(UpdateBoxError::NoTokens)?
            .token_id;
        if update_token_id != inputs.update_nft_token_id.token_id() {
            return Err(UpdateBoxError::IncorrectUpdateTokenId(update_token_id));
        }
        let contract =
            UpdateContract::from_ergo_tree(b.ergo_tree.clone(), &inputs.contract_inputs)?;

        Ok(Self {
            ergo_box: b,
            contract,
        })
    }
    pub fn ergo_tree(&self) -> ErgoTree {
        self.contract.ergo_tree()
    }
    pub fn update_nft(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
    }
    pub fn ballot_token_id(&self) -> TokenId {
        self.contract.ballot_token_id()
    }
    pub fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }
    pub fn min_votes(&self) -> u32 {
        self.contract.min_votes() as u32
    }
}

#[derive(Debug, Clone)]
pub struct UpdateBoxWrapperInputs {
    pub contract_inputs: UpdateContractInputs,
    pub update_nft_token_id: UpdateTokenId,
}

impl UpdateBoxWrapperInputs {
    pub fn build_with(
        update_contract_parameters: UpdateContractParameters,
        pool_nft_token_id: PoolTokenId,
        ballot_token_id: BallotTokenId,
        update_nft_token_id: UpdateTokenId,
    ) -> Result<Self, UpdateContractError> {
        let contract_inputs = UpdateContractInputs::build_with(
            update_contract_parameters,
            pool_nft_token_id,
            ballot_token_id,
        )?;
        Ok(UpdateBoxWrapperInputs {
            contract_inputs,
            update_nft_token_id,
        })
    }

    pub fn checked_load(
        update_contract_parameters: UpdateContractParameters,
        pool_nft_token_id: PoolTokenId,
        ballot_token_id: BallotTokenId,
        update_nft_token_id: UpdateTokenId,
    ) -> Result<Self, UpdateContractError> {
        let contract_inputs = UpdateContractInputs::checked_load(
            update_contract_parameters,
            pool_nft_token_id,
            ballot_token_id,
        )?;
        Ok(UpdateBoxWrapperInputs {
            contract_inputs,
            update_nft_token_id,
        })
    }
}

impl From<UpdateBoxWrapper> for ErgoBox {
    fn from(w: UpdateBoxWrapper) -> Self {
        w.ergo_box.clone()
    }
}
