use std::convert::{TryFrom, TryInto};
use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilder,
        ergo_box::box_builder::ErgoBoxCandidateBuilderError,
        transaction::unsigned::UnsignedTransaction,
    },
    ergo_chain_types::blake2b256_hash,
    ergo_chain_types::{Digest32, DigestNError},
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{
        address::{Address, AddressEncoder, AddressEncoderError, NetworkPrefix},
        ergo_box::{box_value::BoxValue, NonMandatoryRegisterId},
        token::{Token, TokenAmount, TokenId},
    },
    ergotree_ir::mir::constant::Constant,
    ergotree_ir::serialization::SigmaSerializable,
    wallet::{
        box_selector::{BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;

use crate::{
    box_kind::{
        make_local_ballot_box_candidate, make_pool_box_candidate, BallotBox, BallotBoxWrapper,
        PoolBox,
    },
    cli_commands::ergo_explorer_transaction_link,
    contracts::ballot::BallotContract,
    contracts::pool::PoolContract,
    node_interface::{current_block_height, get_wallet_status, sign_and_submit_transaction},
    oracle_config::ORACLE_CONFIG,
    oracle_state::{
        BallotBoxesSource, LocalBallotBoxSource, OraclePool, PoolBoxSource, StageError,
        UpdateBoxSource,
    },
    wallet::WalletDataSource,
};
use derive_more::From;
use thiserror::Error;

#[derive(Debug, Error, From)]
pub enum UpdatePoolError {
    #[error("Update pool: Not enough votes, expected {0}, found {1}")]
    NotEnoughVotes(usize, usize),
    #[error("Update pool: Pool parameters (refresh NFT, update NFT) unchanged")]
    PoolUnchanged,
    #[error("Update pool: ErgoBoxCandidateBuilderError {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("Update pool: box selector error {0}")]
    BoxSelector(BoxSelectorError),
    #[error("Update pool: tx builder error {0}")]
    TxBuilder(TxBuilderError),
    #[error("Update pool: stage error {0}")]
    StageError(StageError),
    #[error("Update pool: node error {0}")]
    Node(NodeError)
}


// TODO: Convert to Result
fn build_update_pool_box_tx(
    pool_box_source: &dyn PoolBoxSource,
    ballot_boxes: &dyn BallotBoxesSource,
    wallet: &dyn WalletDataSource,
    update_box: &dyn UpdateBoxSource,
    new_pool_contract: PoolContract,
    height: u32,
    change_address: Address,
) -> Result<UnsignedTransaction, UpdatePoolError> {
    let min_votes = update_box.get_update_box()?.min_votes();
    let old_pool_box = pool_box_source.get_pool_box()?;
    let update_box = update_box.get_update_box()?;
    let pool_box_hash = Constant::from(blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    ));
    // Find ballot boxes that are voting for the new pool hash
    let vote_ballot_boxes: Vec<BallotBoxWrapper> = ballot_boxes
        .get_ballot_boxes()?
        .into_iter()
        .filter(|ballot_box| {
            let ballot_box = ballot_box.get_box();
            ballot_box
                .additional_registers
                .get(NonMandatoryRegisterId::R6)
                == Some(&pool_box_hash)
                && ballot_box
                    .additional_registers
                    .get(NonMandatoryRegisterId::R7)
                    == Some(&old_pool_box.reward_token().token_id.into())
                && ballot_box
                    .additional_registers
                    .get(NonMandatoryRegisterId::R8)
                    == Some(&(*old_pool_box.reward_token().amount.as_u64() as i64).into())
        })
        .collect();
    if vote_ballot_boxes.len() < min_votes as usize {
        return Err(UpdatePoolError::NotEnoughVotes(min_votes as usize, vote_ballot_boxes.len()));
    }

    let pool_box_candidate = make_pool_box_candidate(
        &new_pool_contract,
        old_pool_box.rate() as i64,
        old_pool_box.epoch_counter() as i32,
        old_pool_box.pool_nft_token(),
        old_pool_box.reward_token(),
        BoxValue::SAFE_USER_MIN,
        old_pool_box.get_box().creation_height, // creation info must be preserved
    )?;
    let mut update_box_candidate =
        ErgoBoxCandidateBuilder::new(BoxValue::SAFE_USER_MIN, update_box.ergo_tree(), height);
    update_box_candidate.add_token(update_box.update_nft());
    let update_box_candidate = update_box_candidate.build()?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    // Storage rent for update box + pool box and all ballot boxes being spent
    let target_balance = BoxValue::SAFE_USER_MIN
        .checked_mul_u32((vote_ballot_boxes.len() + 2) as u32)
        .unwrap();
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector
        .select(unspent_boxes, target_balance, &[])?;
    let mut input_boxes = vec![old_pool_box.get_box().clone(), update_box.get_box().clone()];
    input_boxes.extend(
        vote_ballot_boxes
            .iter()
            .map(|ballot_box| ballot_box.get_box())
            .cloned(),
    );
    input_boxes.extend_from_slice(&*selection.boxes.as_vec());
    let box_selection = BoxSelection {
        boxes: input_boxes.try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };

    let mut outputs = vec![pool_box_candidate, update_box_candidate];
    for ballot_box in vote_ballot_boxes.iter() {
        let mut ballot_box_candidate = ErgoBoxCandidateBuilder::new(
            ballot_box.get_box().value, // value must be preserved or increased
            ballot_box.contract().ergo_tree(),
            height,
        );
        ballot_box_candidate.add_token(ballot_box.ballot_token());
        ballot_box_candidate.set_register_value(
            NonMandatoryRegisterId::R4,
            (*ballot_box.ballot_token_owner().h).clone().into(),
        );
        outputs.push(ballot_box_candidate.build()?);
    }

    let mut tx_builder = TxBuilder::new(
        box_selection,
        outputs.clone(),
        height,
        BoxValue::SAFE_USER_MIN,
        change_address,
        BoxValue::MIN,
    );

    for (i, input_ballot) in vote_ballot_boxes.iter().enumerate() {
        tx_builder.set_context_extension(
            input_ballot.get_box().box_id(),
            ContextExtension {
                values: [(0, ((i + 2) as i32).into())].iter().cloned().collect(), // first 2 outputs are pool and update box, ballot indexes start at 2
            },
        )
    }
    Ok(tx_builder.build()?)
}

#[cfg(test)]
mod tests {
    use ergo_lib::{
        chain::{
            ergo_box::box_builder::ErgoBoxCandidateBuilder, ergo_state_context::ErgoStateContext,
            transaction::TxId,
        },
        ergo_chain_types::{blake2b256_hash, Digest32},
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::{
            chain::{
                address::AddressEncoder,
                ergo_box::{box_value::BoxValue, BoxTokens, ErgoBox},
                token::{Token, TokenId},
            },
            serialization::{sigma_byte_writer::SigmaByteWriter, SigmaSerializable},
        },
        wallet::{signing::TransactionContext, Wallet},
    };
    use sigma_test_util::force_any_val;
    use std::convert::{TryFrom, TryInto};

    use crate::{
        box_kind::{
            make_local_ballot_box_candidate, make_pool_box_candidate, BallotBox, BallotBoxWrapper,
            PoolBox, PoolBoxWrapper, UpdateBoxWrapper,
        },
        contracts::{ballot::BallotContract, pool::PoolContract, update::UpdateContract},
        pool_commands::test_utils::{
            find_input_boxes, make_wallet_unspent_box, BallotBoxMock, BallotBoxesMock, PoolBoxMock,
            UpdateBoxMock, WalletDataMock,
        },
        wallet::WalletDataSource,
    };

    use super::build_update_pool_box_tx;

    fn force_any_tokenid() -> TokenId {
        use proptest::strategy::Strategy;
        proptest::arbitrary::any_with::<TokenId>(
            ergo_lib::ergotree_ir::chain::token::arbitrary::ArbTokenIdParam::Arbitrary,
        )
        .new_tree(&mut Default::default())
        .unwrap()
        .current()
    }
    #[test]
    fn test_update_pool_box() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;

        let ballot_token_id = force_any_tokenid();
        let pool_nft_token_id = force_any_tokenid();
        let update_nft_token_id = force_any_tokenid();
        let reward_token_id = force_any_tokenid();
        let reward_tokens = Token {
            token_id: reward_token_id,
            amount: 1500.try_into().unwrap(),
        };
        let update_contract = UpdateContract::new()
            .with_min_votes(1)
            .with_pool_nft_token_id(pool_nft_token_id.clone())
            .with_ballot_token_id(ballot_token_id.clone());
        let mut update_box_candidate = ErgoBoxCandidateBuilder::new(
            BoxValue::SAFE_USER_MIN,
            update_contract.ergo_tree(),
            height,
        );
        update_box_candidate.add_token(Token {
            token_id: update_nft_token_id.clone(),
            amount: 1.try_into().unwrap(),
        });
        let update_box = ErgoBox::from_box_candidate(
            &update_box_candidate.build().unwrap(),
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();

        let pool_contract = PoolContract::new()
            .with_refresh_nft_token_id(force_any_tokenid())
            .with_update_nft_token_id(update_nft_token_id.clone());
        let pool_box_candidate = make_pool_box_candidate(
            &pool_contract,
            0,
            0,
            Token {
                token_id: pool_nft_token_id.clone(),
                amount: 1.try_into().unwrap(),
            },
            reward_tokens.clone(),
            BoxValue::SAFE_USER_MIN,
            height,
        )
        .unwrap();
        let pool_box =
            ErgoBox::from_box_candidate(&pool_box_candidate, force_any_val::<TxId>(), 0).unwrap();

        let new_refresh_token_id = force_any_tokenid();
        let new_pool_contract = PoolContract::new()
            .with_refresh_nft_token_id(new_refresh_token_id)
            .with_update_nft_token_id(update_nft_token_id.clone());

        let pool_box_bytes = new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap();
        let pool_box_hash = blake2b256_hash(&pool_box_bytes);

        let ballot_contract = BallotContract::new()
            .with_min_storage_rent(*BoxValue::SAFE_USER_MIN.as_u64())
            .with_update_nft_token_id(update_nft_token_id);
        let mut ballot_boxes = vec![];

        for _ in 0..5 {
            let secret = DlogProverInput::random();
            let ballot_box_candidate = make_local_ballot_box_candidate(
                &ballot_contract,
                secret.public_image(),
                update_box.creation_height,
                Token {
                    token_id: ballot_token_id.clone(),
                    amount: 1.try_into().unwrap(),
                },
                pool_box_hash.clone(),
                reward_tokens.clone(),
                BoxValue::SAFE_USER_MIN,
                height,
            )
            .unwrap();
            let ballot_box =
                ErgoBox::from_box_candidate(&ballot_box_candidate, force_any_val::<TxId>(), 0)
                    .unwrap();
            ballot_boxes.push(ballot_box.try_into().unwrap());
        }
        let ballot_boxes_mock = BallotBoxesMock { ballot_boxes };

        let secret = DlogProverInput::random();
        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BoxValue::SAFE_USER_MIN
                .checked_mul_u32(4_000_000_000)
                .unwrap(),
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let update_mock = UpdateBoxMock {
            update_box: UpdateBoxWrapper::try_from(update_box).unwrap(),
        };
        let pool_mock = PoolBoxMock {
            pool_box: PoolBoxWrapper::try_from(pool_box).unwrap(),
        };

        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();

        let update_tx = build_update_pool_box_tx(
            &pool_mock,
            &ballot_boxes_mock,
            &wallet_mock,
            &update_mock,
            new_pool_contract,
            height + 1,
            change_address,
        )
        .unwrap();

        let mut input_boxes = vec![
            pool_mock.pool_box.get_box().clone(),
            update_mock.update_box.get_box().clone(),
        ];
        input_boxes.extend(
            ballot_boxes_mock
                .ballot_boxes
                .iter()
                .map(|ballot_box| ballot_box.get_box().clone()),
        );
        input_boxes.extend_from_slice(&wallet_mock.unspent_boxes);
        let tx_context = TransactionContext::new(update_tx, input_boxes, vec![]).unwrap();
        wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
