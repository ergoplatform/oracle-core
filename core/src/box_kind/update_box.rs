use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use thiserror::Error;

use crate::contracts::update::UpdateContract;
use crate::contracts::update::UpdateContractError;
use crate::contracts::update::UpdateContractInputs;
use crate::contracts::update::UpdateContractParameters;

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
pub struct UpdateBoxWrapper(ErgoBox, UpdateContract);

impl UpdateBoxWrapper {
    pub fn new(b: ErgoBox, inputs: &UpdateBoxWrapperInputs) -> Result<Self, UpdateBoxError> {
        let update_token_id = b
            .tokens
            .as_ref()
            .ok_or(UpdateBoxError::NoTokens)?
            .get(0)
            .ok_or(UpdateBoxError::NoTokens)?
            .token_id
            .clone();
        if update_token_id != inputs.update_nft_token_id {
            return Err(UpdateBoxError::IncorrectUpdateTokenId(update_token_id));
        }
        let contract =
            UpdateContract::from_ergo_tree(b.ergo_tree.clone(), &inputs.contract_inputs)?;

        Ok(Self(b, contract))
    }
    pub fn ergo_tree(&self) -> ErgoTree {
        self.1.ergo_tree()
    }
    pub fn update_nft(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(0).unwrap().clone()
    }
    pub fn ballot_token_id(&self) -> TokenId {
        self.1.ballot_token_id().clone()
    }
    pub fn get_box(&self) -> &ErgoBox {
        &self.0
    }
    pub fn min_votes(&self) -> u32 {
        self.1.min_votes() as u32
    }
}

#[derive(Debug, Clone)]
pub struct UpdateBoxWrapperInputs {
    pub contract_inputs: UpdateContractInputs,
    pub update_nft_token_id: TokenId,
}

impl UpdateBoxWrapperInputs {
    pub fn create(
        update_contract_parameters: UpdateContractParameters,
        pool_nft_token_id: TokenId,
        ballot_token_id: TokenId,
        update_nft_token_id: TokenId,
    ) -> Result<Self, UpdateContractError> {
        let contract_inputs = UpdateContractInputs::create(
            update_contract_parameters,
            pool_nft_token_id,
            ballot_token_id,
        )?;
        Ok(UpdateBoxWrapperInputs {
            contract_inputs,
            update_nft_token_id,
        })
    }

    pub fn load(
        update_contract_parameters: UpdateContractParameters,
        pool_nft_token_id: TokenId,
        ballot_token_id: TokenId,
        update_nft_token_id: TokenId,
    ) -> Result<Self, UpdateContractError> {
        let contract_inputs = UpdateContractInputs::load(
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
        w.0.clone()
    }
}
