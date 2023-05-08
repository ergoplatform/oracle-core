use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use thiserror::Error;

use crate::spec_token::RewardTokenId;
use crate::spec_token::SpecToken;

#[derive(Debug, Error)]
pub enum BuybackBoxError {}

pub struct BuybackBoxWrapper {
    ergo_box: ErgoBox,
    reward_token_id: RewardTokenId,
}

#[allow(clippy::todo)]
impl BuybackBoxWrapper {
    pub fn new(ergo_box: ErgoBox, reward_token_id: RewardTokenId) -> Result<Self, BuybackBoxError> {
        Ok(Self {
            ergo_box,
            reward_token_id,
        })
    }

    pub fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    pub fn reward_token(&self) -> Option<SpecToken<RewardTokenId>> {
        todo!()
    }

    pub fn new_without_reward_token(&self) -> ErgoBoxCandidate {
        todo!()
    }
}
