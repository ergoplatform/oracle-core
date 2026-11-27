use std::vec;

use crate::oracle_types::BlockHeight;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::token::Token;
use thiserror::Error;

use crate::spec_token::RewardTokenId;
use crate::spec_token::SpecToken;
use crate::spec_token::TokenIdKind;

#[derive(Debug, Error)]
pub enum BuybackBoxError {}

#[derive(Debug, Clone)]
pub struct BuybackBoxWrapper {
    ergo_box: ErgoBox,
    reward_token_id: RewardTokenId,
}

#[allow(clippy::todo)]
impl BuybackBoxWrapper {
    pub fn new(ergo_box: ErgoBox, reward_token_id: RewardTokenId) -> Self {
        Self {
            ergo_box,
            reward_token_id,
        }
    }

    pub fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    pub fn reward_token(&self) -> Option<SpecToken<RewardTokenId>> {
        self.ergo_box
            .tokens
            .as_ref()?
            .get(1)
            .filter(|token| token.token_id == self.reward_token_id.token_id())
            .map(|token| SpecToken {
                token_id: self.reward_token_id.clone(),
                amount: token.amount,
            })
    }

    pub fn new_with_one_reward_token(&self, creation_height: BlockHeight) -> ErgoBoxCandidate {
        let single_reward_token = Token {
            token_id: self.reward_token_id.token_id(),
            amount: 1.try_into().unwrap(),
        };

        // take buyback nft and at least one reward token
        let tokens = vec![
            self.ergo_box
                .tokens
                .as_ref()
                .unwrap()
                .get(0)
                .unwrap()
                .clone(),
            single_reward_token,
        ]
        .try_into()
        .unwrap();

        ErgoBoxCandidate {
            value: self.ergo_box.value,
            ergo_tree: self.ergo_box.ergo_tree.clone(),
            tokens: Some(tokens),
            additional_registers: self.ergo_box.additional_registers.clone(),
            creation_height: creation_height.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::ergo_chain_types::Digest32;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::{BoxTokens, ErgoBox, NonMandatoryRegisters};
    use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::mir::expr::Expr;
    use sigma_test_util::force_any_val;

    use super::*;

    fn make_box(tokens: Option<BoxTokens>) -> ErgoBox {
        let secret = force_any_val::<DlogProverInput>();
        let expr: Expr = Constant::from(secret.public_image()).into();
        ErgoBox::new(
            BoxValue::SAFE_USER_MIN,
            ErgoTree::try_from(expr).unwrap(),
            tokens,
            NonMandatoryRegisters::empty(),
            1,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    #[test]
    fn test_buyback_reward_token_validation() {
        let buyback_nft_id = TokenId::from(Digest32::from([1u8; 32]));
        let expected_reward_id =
            RewardTokenId::from_token_id_unchecked(TokenId::from(Digest32::from([2u8; 32])));
        let other_token_id = TokenId::from(Digest32::from([3u8; 32]));

        let no_tokens_box = BuybackBoxWrapper::new(make_box(None), expected_reward_id.clone());
        assert!(no_tokens_box.reward_token().is_none());

        let only_nft = BuybackBoxWrapper::new(
            make_box(Some(
                vec![Token {
                    token_id: buyback_nft_id,
                    amount: 1u64.try_into().unwrap(),
                }]
                .try_into()
                .unwrap(),
            )),
            expected_reward_id.clone(),
        );
        assert!(only_nft.reward_token().is_none());

        let wrong_reward_token = BuybackBoxWrapper::new(
            make_box(Some(
                vec![
                    Token {
                        token_id: buyback_nft_id,
                        amount: 1u64.try_into().unwrap(),
                    },
                    Token {
                        token_id: other_token_id,
                        amount: 50u64.try_into().unwrap(),
                    },
                ]
                .try_into()
                .unwrap(),
            )),
            expected_reward_id.clone(),
        );
        assert!(wrong_reward_token.reward_token().is_none());

        let matching_reward_token = BuybackBoxWrapper::new(
            make_box(Some(
                vec![
                    Token {
                        token_id: buyback_nft_id,
                        amount: 1u64.try_into().unwrap(),
                    },
                    Token {
                        token_id: expected_reward_id.token_id(),
                        amount: 50u64.try_into().unwrap(),
                    },
                ]
                .try_into()
                .unwrap(),
            )),
            expected_reward_id.clone(),
        );
        let reward = matching_reward_token.reward_token().unwrap();
        assert_eq!(reward.token_id, expected_reward_id);
        assert_eq!(*reward.amount.as_u64(), 50);
    }
}
