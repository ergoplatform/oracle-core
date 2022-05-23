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
    contracts::{pool::PoolContract, refresh::RefreshContract},
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

pub struct BootstrapInput<'a> {
    pub state: BootstrapState,
    pub wallet: &'a dyn WalletDataSource,
    pub wallet_sign: &'a dyn WalletSign,
    pub submit_tx: &'a dyn SubmitTransaction,
    pub wallet_pk: ProveDlog,
    pub tx_fee: BoxValue,
    pub erg_value_per_box: BoxValue,
    pub change_address: Address,
    pub height: u32,
    pub initial_datapoint: i64,
    pub epoch_length: u32,
    pub buffer: u32,
    pub total_oracles: u32,
    pub min_data_points: u32,
    pub max_deviation_percent: u32,
    pub total_ballots: u32,
    pub min_votes: u32,
}

/// Perform and submit to the mempool the chained-transaction to boostrap the oracle pool. We first
/// mint the oracle-pool tokens then create the pool and refresh boxes as described in EIP-23:
/// https://github.com/ergoplatform/eips/blob/eip23/eip-0023.md#tokens
pub fn perform_bootstrap_chained_transaction(
    input: BootstrapInput,
) -> Result<MintedTokenIds, BootstrapError> {
    let BootstrapInput {
        state,
        wallet,
        wallet_sign,
        submit_tx,
        wallet_pk,
        tx_fee,
        erg_value_per_box,
        change_address,
        height,
        initial_datapoint,
        ..
    } = input;

    // We can calculate the amount of ERGs necessary to effect this chained-transaction upfront.
    // We're going to mint 6 distinct types of tokens and create the pool and refresh boxes as
    // described in EIP-23. The minting of each type of token requires a distinct transaction, so we
    // need 8 transactions in total. We assume that the resulting token-holding boxes generated from
    // these transactions each has a box value of `erg_value_per_box`. Similarly the pool and
    // refresh boxes will also hold `erg_value_per_box`.
    //
    // Now define `E_i = i*(erg_value_per_box + tx_free)` for `i = 1,2,.., 8`. `E_i` represents the
    // amount of ERGs necessary to effect `i` remaining transactions.
    //
    // So we require a total ERG value of `E_8 = 8*(erg_value_per_box + tx_free)`
    //
    // The chain transaction is structured as follows:
    //   * First sweep the unspent boxes of the wallet for a target balance of `E_8`. Denote these
    //     input boxes by `I_1`.
    //
    //   * Mint the first token with `I_1` as input, resulting in two output boxes:
    //      - `B_1_token` containing the minted token and `ergo_value_per_box`
    //      - `B_1_remaining` containing `E_7` in ERG value.
    //
    //   * Mint the second token with input boxes containing `B_1_remaining`, resulting in two
    //     output boxes:
    //      - `B_2_token` containing the minted token and `ergo_value_per_box`
    //      - `B_2_remaining` containing `E_6` in ERG value.
    //
    // And so on.

    // This variable represents the index `i` described above.
    let mut num_transactions_left = 8;

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

    // This closure computes `E_{num_transactions_left}`.
    let calc_target_balance = |num_transactions_left| {
        let b = erg_value_per_box.checked_mul_u32(num_transactions_left)?;
        let fees = tx_fee.checked_mul_u32(num_transactions_left)?;
        b.checked_add(&fees)
    };

    // Effect a single transaction that mints a token with given details, as described in comments
    // at the beginning.
    let mint_token = |input_boxes: Vec<ErgoBox>,
                      num_transactions_left: &mut u32,
                      token_name,
                      token_desc,
                      token_amount|
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

        // Build box for remaining funds
        builder = ErgoBoxCandidateBuilder::new(
            calc_target_balance(*num_transactions_left - 1)?,
            ergo_tree.clone(),
            height,
        );
        let output_with_token = builder.build()?;
        output_candidates.push(output_with_token.clone());

        let inputs = box_selection.boxes.clone();
        let tx_builder = TxBuilder::new(
            box_selection,
            output_candidates,
            height,
            tx_fee,
            change_address.clone(),
            BoxValue::MIN,
        );
        let mint_token_tx = tx_builder.build()?;
        let signed_tx = wallet_sign.sign_transaction_with_inputs(&mint_token_tx, inputs, None)?;
        *num_transactions_left -= 1;
        Ok((token, signed_tx))
    };

    // Mint pool NFT token --------------------------------------------------------------------------
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let target_balance = calc_target_balance(num_transactions_left)?;
    let box_selector = SimpleBoxSelector::new();
    let box_selection = box_selector.select(unspent_boxes.clone(), target_balance, &[])?;

    let (pool_nft_token, signed_mint_pool_nft_tx) = mint_token(
        box_selection.boxes.as_vec().clone(),
        &mut num_transactions_left,
        state.pool_nft.name.clone(),
        state.pool_nft.description.clone(),
        1.try_into().unwrap(),
    )?;
    // Mint refresh NFT token ----------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_pool_nft_tx.outputs.clone());
    let (refresh_nft_token, signed_mint_refresh_nft_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        state.refresh_nft.name.clone(),
        state.refresh_nft.description.clone(),
        1.try_into().unwrap(),
    )?;

    // Mint update NFT token -----------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_refresh_nft_tx.outputs.clone());
    let (update_nft_token, signed_mint_update_nft_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        state.update_nft.name.clone(),
        state.update_nft.description.clone(),
        1.try_into().unwrap(),
    )?;

    // Mint oracle tokens --------------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_update_nft_tx.outputs.clone());
    let (oracle_token, signed_mint_oracle_tokens_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        state.oracle_tokens.name.clone(),
        state.oracle_tokens.description.clone(),
        state.oracle_tokens.quantity.try_into().unwrap(),
    )?;

    // Mint ballot tokens --------------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_oracle_tokens_tx.outputs.clone());
    let (ballot_token, signed_mint_ballot_tokens_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        state.ballot_tokens.name.clone(),
        state.ballot_tokens.description.clone(),
        state.ballot_tokens.quantity.try_into().unwrap(),
    )?;

    // Mint reward tokens --------------------------------------------------------------------------
    let inputs = filter_tx_outputs(signed_mint_ballot_tokens_tx.outputs.clone());
    let (reward_token, signed_mint_reward_tokens_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        state.reward_tokens.name.clone(),
        state.reward_tokens.description.clone(),
        state.reward_tokens.quantity.try_into().unwrap(),
    )?;

    // Create pool box -----------------------------------------------------------------------------
    let pool_contract = PoolContract::new()
        .with_refresh_nft_token_id(refresh_nft_token.token_id.clone())
        .with_update_nft_token_id(update_nft_token.token_id.clone());

    let mut builder =
        ErgoBoxCandidateBuilder::new(erg_value_per_box, pool_contract.ergo_tree(), height);
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId::{R4, R5};
    builder.set_register_value(R4, initial_datapoint.into());
    builder.set_register_value(R5, 1_i64.into());
    builder.add_token(pool_nft_token.clone());

    let mut output_candidates = vec![builder.build()?];

    // Build box for remaining funds
    builder = ErgoBoxCandidateBuilder::new(
        calc_target_balance(num_transactions_left - 1)?,
        ergo_tree.clone(),
        height,
    );
    output_candidates.push(builder.build()?);

    let target_balance = calc_target_balance(num_transactions_left)?;
    let box_selector = SimpleBoxSelector::new();
    let mut inputs = filter_tx_outputs(signed_mint_reward_tokens_tx.outputs.clone());

    // Need to find the box containing the pool NFT, and transfer this token to the pool box.
    let box_with_pool_nft = signed_mint_pool_nft_tx
        .outputs
        .iter()
        .find(|b| {
            if let Some(tokens) = &b.tokens {
                tokens.iter().any(|t| t.token_id == pool_nft_token.token_id)
            } else {
                false
            }
        })
        .unwrap()
        .clone();
    inputs.push(box_with_pool_nft);

    let box_selection = box_selector.select(inputs, target_balance, &[pool_nft_token.clone()])?;
    let inputs = box_selection.boxes.clone();
    let tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        height,
        tx_fee,
        change_address.clone(),
        BoxValue::MIN,
    );
    let pool_box_tx = tx_builder.build()?;
    let signed_pool_box_tx =
        wallet_sign.sign_transaction_with_inputs(&pool_box_tx, inputs, None)?;
    num_transactions_left -= 1;

    // Create refresh box --------------------------------------------------------------------------
    let BootstrapInput {
        epoch_length,
        buffer,
        min_data_points,
        max_deviation_percent,
        ..
    } = input;

    let refresh_contract = RefreshContract::new()
        .with_oracle_nft_token_id(oracle_token.token_id.clone())
        .with_pool_nft_token_id(pool_nft_token.token_id.clone())
        .with_epoch_length(epoch_length)
        .with_buffer(buffer)
        .with_min_data_points(min_data_points)
        .with_max_deviation_percent(max_deviation_percent);

    let mut builder =
        ErgoBoxCandidateBuilder::new(erg_value_per_box, refresh_contract.ergo_tree(), height);

    builder.add_token(refresh_nft_token.clone());

    let single_reward_token = Token {
        token_id: reward_token.token_id.clone(),
        amount: 1.try_into().unwrap(),
    };
    builder.add_token(single_reward_token.clone());

    let output_candidates = vec![builder.build()?];

    let target_balance = calc_target_balance(num_transactions_left)?;
    let box_selector = SimpleBoxSelector::new();
    let mut inputs = filter_tx_outputs(signed_mint_reward_tokens_tx.outputs.clone());

    // Need to find the box containing the refresh NFT, and transfer this token to the refresh box.
    let box_with_refresh_nft = signed_mint_refresh_nft_tx
        .outputs
        .iter()
        .find(|b| {
            if let Some(tokens) = &b.tokens {
                tokens
                    .iter()
                    .any(|t| t.token_id == refresh_nft_token.token_id)
            } else {
                false
            }
        })
        .unwrap()
        .clone();
    inputs.push(box_with_refresh_nft);

    let box_selection = box_selector.select(
        inputs,
        target_balance,
        &[refresh_nft_token.clone(), single_reward_token],
    )?;
    let inputs = box_selection.boxes.clone();
    let tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        height,
        tx_fee,
        change_address.clone(),
        BoxValue::MIN,
    );
    let refresh_box_tx = tx_builder.build()?;
    let signed_refresh_box_tx =
        wallet_sign.sign_transaction_with_inputs(&refresh_box_tx, inputs, None)?;

    // ---------------------------------------------------------------------------------------------
    submit_tx.submit_transaction(&signed_mint_pool_nft_tx)?;
    submit_tx.submit_transaction(&signed_mint_refresh_nft_tx)?;
    submit_tx.submit_transaction(&signed_mint_update_nft_tx)?;
    submit_tx.submit_transaction(&signed_mint_oracle_tokens_tx)?;
    submit_tx.submit_transaction(&signed_mint_ballot_tokens_tx)?;
    submit_tx.submit_transaction(&signed_mint_reward_tokens_tx)?;
    submit_tx.submit_transaction(&signed_pool_box_tx)?;
    submit_tx.submit_transaction(&signed_refresh_box_tx)?;

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
        wallet: Wallet,
        guard: ErgoTree,
    }

    impl WalletSign for TestWallet {
        fn sign_transaction_with_inputs(
            &self,
            unsigned_tx: &UnsignedTransaction,
            inputs: TxIoVec<ErgoBox>,
            data_boxes: Option<TxIoVec<ErgoBox>>,
        ) -> Result<ergo_lib::chain::transaction::Transaction, NodeError> {
            let tx = self
                .wallet
                .sign_transaction(
                    TransactionContext::new(unsigned_tx.clone(), inputs, data_boxes).unwrap(),
                    &self.ctx,
                    None,
                )
                .unwrap();
            Ok(tx)
        }
    }

    #[test]
    fn test_bootstrap() {
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
        let change_address =
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
        let _ = perform_bootstrap_chained_transaction(BootstrapInput {
            state,
            wallet: &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
            },
            wallet_sign: &mut TestWallet {
                ctx,
                wallet,
                guard: ergo_tree,
            },
            submit_tx: &SubmitTxMock {},
            wallet_pk: secret.public_image(),
            tx_fee: BoxValue::SAFE_USER_MIN,
            erg_value_per_box: BoxValue::SAFE_USER_MIN,
            change_address,
            height,
            initial_datapoint: 200,
            epoch_length: 30,
            buffer: 4,
            total_oracles: 15,
            min_data_points: 4,
            max_deviation_percent: 5,
            total_ballots: 15,
            min_votes: 6,
        })
        .unwrap();
    }
}
