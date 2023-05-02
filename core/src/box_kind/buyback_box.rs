use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;

use crate::spec_token::BuybackTokenId;
use crate::spec_token::RewardTokenId;
use crate::spec_token::SpecToken;

pub struct BuybackBoxWrapper {}

impl BuybackBoxWrapper {
    pub fn get_box(&self) -> ErgoBox {
        unimplemented!()
    }

    pub fn buyback_nft(&self) -> SpecToken<BuybackTokenId> {
        unimplemented!()
    }

    pub fn reward_token(&self) -> Option<SpecToken<RewardTokenId>> {
        unimplemented!()
    }
}
