//! Bootstrap a new oracle pool
use std::convert::{TryFrom, TryInto};

use derive_more::From;
use ergo_lib::{
    chain::{
        ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
        transaction::Transaction,
    },
    ergotree_ir::{
        chain::{
            address::Address,
            ergo_box::{
                box_value::{BoxValue, BoxValueError},
                ErgoBox,
            },
            token::{Token, TokenId},
        },
        ergo_tree::ErgoTree,
        mir::{constant::Constant, expr::Expr},
        sigma_protocol::sigma_boolean::ProveDlog,
    },
    wallet::{
        box_selector::{BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    node_interface::SubmitTransaction,
    wallet::{WalletDataSource, WalletSign},
};

#[derive(Deserialize)]
pub struct BootstrapState {
    /// Optionally set a prefix for all token names.
    pub oracle_pool_name_prefix: Option<String>,
    pub pool_nft: NftMintDetails,
    pub refresh_nft: NftMintDetails,
    pub update_nft: NftMintDetails,
    pub oracle_tokens: TokenMintDetails,
    pub ballot_tokens: TokenMintDetails,
    pub reward_tokens: TokenMintDetails,
}

#[derive(Deserialize)]
pub struct TokenMintDetails {
    pub name: String,
    pub description: String,
    pub quantity: u64,
}

#[derive(Deserialize)]
pub struct NftMintDetails {
    pub name: String,
    pub description: String,
}

pub struct MintedTokenIds {
    pub pool_nft: TokenId,
    pub refresh_nft: TokenId,
    pub update_nft: TokenId,
    pub oracle_token: TokenId,
    pub ballot_token: TokenId,
    pub reward_token: TokenId,
}

#[derive(Debug, Error, From)]
pub enum BootstrapError {
    #[error("tx builder error: {0}")]
    TxBuilder(TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("node error: {0}")]
    Node(NodeError),
    #[error("box selector error: {0}")]
    BoxSelector(BoxSelectorError),
    #[error("box value error: {0}")]
    BoxValue(BoxValueError),
}

pub struct MintTokensInput<'a> {
    pub state: BootstrapState,
    pub wallet: &'a dyn WalletDataSource,
    pub wallet_sign: &'a mut dyn WalletSign,
    pub submit_tx: &'a dyn SubmitTransaction,
    pub wallet_pk: ProveDlog,
    pub tx_fee: BoxValue,
    pub erg_value_per_box: BoxValue,
    pub change_address: Address,
    pub height: u32,
}

/// Mint the oracle-pool tokens as described in EIP-23: https://github.com/ergoplatform/eips/blob/eip23/eip-0023.md#tokens
pub fn mint_tokens(input: MintTokensInput) -> Result<MintedTokenIds, BootstrapError> {
    let MintTokensInput {
        state,
        wallet,
        wallet_sign,
        submit_tx,
        wallet_pk,
        tx_fee,
        erg_value_per_box,
        change_address,
        height,
    } = input;
    let c: Constant = wallet_pk.into();
    let expr: Expr = c.into();
    let ergo_tree = ErgoTree::try_from(expr).unwrap();
    let guard = ergo_tree.clone();

    // Since we're building a chain of transactions, we need to filter the output boxes of each
    // constituent transaction to be only those that are guarded by our wallet's key.
    let filter_tx_outputs = move |outputs: Vec<ErgoBox>| -> Vec<ErgoBox> {
        outputs
            .clone()
            .into_iter()
            .filter(|b| b.ergo_tree == guard)
            .collect()
    };

    let calc_target_balance = |num_transactions_left| {
        let b = erg_value_per_box.checked_mul_u32(num_transactions_left)?;
        let fees = tx_fee.checked_mul_u32(num_transactions_left)?;
        b.checked_add(&fees)
    };

    let mut mint_tokens = |input_boxes: Vec<ErgoBox>,
                           num_transactions_left: &mut u32,
                           token_name,
                           token_desc,
                           token_amount,
                           build_box_for_remaining_funds|
     -> Result<(Token, Transaction), BootstrapError> {
        let target_balance = calc_target_balance(*num_transactions_left)?;
        let box_selector = SimpleBoxSelector::new();
        let box_selection = box_selector.select(input_boxes, target_balance, &[])?;
        let token = Token {
            token_id: box_selection.boxes.first().box_id().into(),
            amount: token_amount,
        };
        let mut builder =
            ErgoBoxCandidateBuilder::new(erg_value_per_box, ergo_tree.clone(), height);
        builder.mint_token(token.clone(), token_name, token_desc, 1);
        let mut output_candidates = vec![builder.build()?];

        if build_box_for_remaining_funds {
            builder = ErgoBoxCandidateBuilder::new(
                calc_target_balance(*num_transactions_left - 1)?,
                ergo_tree.clone(),
                height,
            );
            output_candidates.push(builder.build()?);
        }
        let tx_builder = TxBuilder::new(
            box_selection,
            output_candidates,
            height,
            tx_fee,
            change_address.clone(),
            BoxValue::MIN,
        );
        let mint_token_tx = tx_builder.build()?;
        let signed_tx = wallet_sign.sign_transaction(&mint_token_tx)?;
        *num_transactions_left -= 1;
        Ok((token, signed_tx))
    };

    // Mint pool NFT token --------------------------------------------------------------------------
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let mut num_token_ids_to_mint = 6;
    let target_balance = calc_target_balance(num_token_ids_to_mint)?;
    let box_selector = SimpleBoxSelector::new();
    let box_selection = box_selector.select(unspent_boxes.clone(), target_balance, &[])?;

    let (pool_nft_token, signed_mint_pool_nft_tx) = mint_tokens(
        box_selection.boxes.as_vec().clone(),
        &mut num_token_ids_to_mint,
        state.pool_nft.name.clone(),
        state.pool_nft.description.clone(),
        1.try_into().unwrap(),
        true,
    )?;

    // Mint refresh NFT token ----------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_pool_nft_tx.outputs.clone());
    let (refresh_nft_token, signed_mint_refresh_nft_tx) = mint_tokens(
        inputs,
        &mut num_token_ids_to_mint,
        state.refresh_nft.name.clone(),
        state.refresh_nft.description.clone(),
        1.try_into().unwrap(),
        true,
    )?;

    // Mint update NFT token -----------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_refresh_nft_tx.outputs.clone());
    let (update_nft_token, signed_mint_update_nft_tx) = mint_tokens(
        inputs,
        &mut num_token_ids_to_mint,
        state.update_nft.name.clone(),
        state.update_nft.description.clone(),
        1.try_into().unwrap(),
        true,
    )?;

    // Mint oracle tokens --------------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_update_nft_tx.outputs.clone());
    let (oracle_token, signed_mint_oracle_tokens_tx) = mint_tokens(
        inputs,
        &mut num_token_ids_to_mint,
        state.oracle_tokens.name.clone(),
        state.oracle_tokens.description.clone(),
        state.oracle_tokens.quantity.try_into().unwrap(),
        true,
    )?;

    // Mint ballot tokens --------------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_oracle_tokens_tx.outputs.clone());
    let (ballot_token, signed_mint_ballot_tokens_tx) = mint_tokens(
        inputs,
        &mut num_token_ids_to_mint,
        state.ballot_tokens.name.clone(),
        state.ballot_tokens.description.clone(),
        state.ballot_tokens.quantity.try_into().unwrap(),
        true,
    )?;

    // Mint reward tokens --------------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_ballot_tokens_tx.outputs.clone());
    let (reward_token, signed_mint_reward_tokens_tx) = mint_tokens(
        inputs,
        &mut num_token_ids_to_mint,
        state.reward_tokens.name.clone(),
        state.reward_tokens.description.clone(),
        state.reward_tokens.quantity.try_into().unwrap(),
        false,
    )?;

    submit_tx.submit_transaction(&signed_mint_pool_nft_tx)?;
    submit_tx.submit_transaction(&signed_mint_refresh_nft_tx)?;
    submit_tx.submit_transaction(&signed_mint_update_nft_tx)?;
    submit_tx.submit_transaction(&signed_mint_oracle_tokens_tx)?;
    submit_tx.submit_transaction(&signed_mint_ballot_tokens_tx)?;
    submit_tx.submit_transaction(&signed_mint_reward_tokens_tx)?;
    Ok(MintedTokenIds {
        pool_nft: pool_nft_token.token_id,
        refresh_nft: refresh_nft_token.token_id,
        update_nft: update_nft_token.token_id,
        oracle_token: oracle_token.token_id,
        ballot_token: ballot_token.token_id,
        reward_token: reward_token.token_id,
    })
}

#[cfg(test)]
mod tests {
    use ergo_lib::{
        chain::{
            ergo_state_context::ErgoStateContext,
            transaction::{unsigned::UnsignedTransaction, TxId, TxIoVec},
        },
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::AddressEncoder,
            ergo_box::{ErgoBox, NonMandatoryRegisters},
        },
        wallet::{signing::TransactionContext, Wallet},
    };
    use sigma_test_util::force_any_val;

    use super::*;
    use crate::commands::test_utils::WalletDataMock;

    struct SubmitTxMock {}

    impl SubmitTransaction for SubmitTxMock {
        fn submit_transaction(
            &self,
            _: &ergo_lib::chain::transaction::Transaction,
        ) -> crate::node_interface::Result<crate::node_interface::TxId> {
            // No-op
            Ok("".into())
        }
    }

    struct TestWallet {
        ctx: ErgoStateContext,
        boxes_to_spend: TxIoVec<ErgoBox>,
        wallet: Wallet,
        guard: ErgoTree,
    }

    impl WalletSign for TestWallet {
        fn sign_transaction(
            &mut self,
            unsigned_tx: &UnsignedTransaction,
        ) -> Result<ergo_lib::chain::transaction::Transaction, NodeError> {
            let tx = self
                .wallet
                .sign_transaction(
                    TransactionContext::new(unsigned_tx.clone(), self.boxes_to_spend.clone(), None)
                        .unwrap(),
                    &self.ctx,
                    None,
                )
                .unwrap();
            self.boxes_to_spend = tx
                .outputs
                .clone()
                .into_iter()
                .filter(|b| b.ergo_tree == self.guard)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            Ok(tx)
        }
    }

    #[test]
    fn test_mint_tokens() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let c: Constant = secret.public_image().into();
        let expr: Expr = c.into();
        let ergo_tree = ErgoTree::try_from(expr).unwrap();

        let value = BoxValue::SAFE_USER_MIN.checked_mul_u32(10000).unwrap();
        let unspent_boxes = vec![ErgoBox::new(
            value,
            ergo_tree.clone(),
            None,
            NonMandatoryRegisters::new(vec![].into_iter().collect()).unwrap(),
            height - 9,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()];
        let change_address = //Address::P2Pk(secret.public_image());
        AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
            .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
            .unwrap();

        let state = BootstrapState {
            oracle_pool_name_prefix: Some("".into()),
            pool_nft: NftMintDetails {
                name: "pool NFT".into(),
                description: "Pool NFT".into(),
            },
            refresh_nft: NftMintDetails {
                name: "refresh NFT".into(),
                description: "refresh NFT".into(),
            },
            update_nft: NftMintDetails {
                name: "update NFT".into(),
                description: "update NFT".into(),
            },
            oracle_tokens: TokenMintDetails {
                name: "oracle token".into(),
                description: "oracle token".into(),
                quantity: 15,
            },
            ballot_tokens: TokenMintDetails {
                name: "ballot token".into(),
                description: "ballot token".into(),
                quantity: 15,
            },
            reward_tokens: TokenMintDetails {
                name: "reward token".into(),
                description: "reward token".into(),
                quantity: 100_000_000,
            },
        };
        let height = ctx.pre_header.height;
        let _ = mint_tokens(MintTokensInput {
            state,
            wallet: &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
            },
            wallet_sign: &mut TestWallet {
                ctx,
                boxes_to_spend: unspent_boxes.clone().try_into().unwrap(),
                wallet,
                guard: ergo_tree,
            },
            submit_tx: &SubmitTxMock {},
            wallet_pk: secret.public_image(),
            tx_fee: BoxValue::SAFE_USER_MIN,
            erg_value_per_box: BoxValue::SAFE_USER_MIN,
            change_address,
            height,
        })
        .unwrap();
    }
}
