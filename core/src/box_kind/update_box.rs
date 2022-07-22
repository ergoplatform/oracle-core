use std::convert::TryFrom;

use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use thiserror::Error;

use crate::contracts::update::UpdateContract;
use crate::contracts::update::UpdateContractError;

#[derive(Debug, Error)]
pub enum UpdateBoxError {
    #[error("oracle box: no tokens found")]
    NoTokens,
    #[error("update contract: {0:?}")]
    UpdateContractError(#[from] UpdateContractError),
}

#[derive(Clone)]
pub struct UpdateBoxWrapper(ErgoBox, UpdateContract);

impl UpdateBoxWrapper {
    pub fn new(b: ErgoBox) -> Result<Self, UpdateBoxError> {
        let _update_token_id = b
            .tokens
            .as_ref()
            .ok_or(UpdateBoxError::NoTokens)?
            .get(0)
            .ok_or(UpdateBoxError::NoTokens)?
            .token_id
            .clone();
        let contract = UpdateContract::from_ergo_tree(b.ergo_tree.clone())?;

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

impl TryFrom<ErgoBox> for UpdateBoxWrapper {
    type Error = UpdateBoxError;

    fn try_from(value: ErgoBox) -> Result<Self, Self::Error> {
        UpdateBoxWrapper::new(value)
    }
}

impl From<UpdateBoxWrapper> for ErgoBox {
    fn from(w: UpdateBoxWrapper) -> Self {
        w.0.clone()
    }
}
