use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;

use crate::spec_token::BuybackTokenId;
use crate::spec_token::RewardTokenId;
use crate::spec_token::SpecToken;

pub struct BuybackBoxWrapper {}

#[allow(clippy::todo)]
impl BuybackBoxWrapper {
    pub fn get_box(&self) -> ErgoBox {
        todo!()
    }

    pub fn buyback_nft(&self) -> SpecToken<BuybackTokenId> {
        todo!()
    }

    pub fn reward_token(&self) -> Option<SpecToken<RewardTokenId>> {
        todo!()
    }

    pub fn without_reward_token(&self) -> ErgoBoxCandidate {
        todo!()
    }
}
