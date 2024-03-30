
use ergo_lib::ergotree_ir::chain::ergo_box::{BoxTokens, ErgoBoxCandidate};
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use thiserror::Error;
use ergo_lib::ergotree_ir::chain::token::Token;

use crate::spec_token::RewardTokenId;
use crate::TokenAmount;

#[derive(Debug, Error)]
pub enum DevRewardBoxError {}

#[derive(Debug, Clone)]
pub struct DevRewardBoxWrapper {
    ergo_box: ErgoBox,
    reward_token_id: RewardTokenId,
}


#[allow(clippy::todo)]
impl DevRewardBoxWrapper {
    pub fn new(ergo_box: ErgoBox, reward_token_id: RewardTokenId) -> Self {
        Self {
            ergo_box,
            reward_token_id,
        }
    }

    pub fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    pub fn add_reward_tokens(&self, to_add: TokenAmount, height: u32) -> ErgoBoxCandidate {
        let tokens_vec = self.ergo_box.tokens.as_ref().unwrap();
        let reward_token = tokens_vec.get(1).unwrap().clone();
        let upd_amt: TokenAmount = (reward_token.amount.as_u64() + to_add.as_u64()).try_into().unwrap();
        let upd_token: Token = (reward_token.token_id, upd_amt).into();

        let tokens: BoxTokens = vec![tokens_vec.get(0).unwrap().clone(), upd_token].try_into().unwrap();

        ErgoBoxCandidate {
            value: self.ergo_box.value,
            ergo_tree: self.ergo_box.ergo_tree.clone(),
            tokens: Some(tokens),
            additional_registers: self.ergo_box.additional_registers.clone(),
            creation_height: height,
        }
    }
}
