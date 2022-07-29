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
        signing::{TransactionContext, TxSigningError},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use std::convert::{TryFrom, TryInto};

use crate::{
    box_kind::{
        make_local_ballot_box_candidate, make_pool_box_candidate, BallotBox, BallotBoxWrapper,
        PoolBox, PoolBoxWrapper,
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
    #[error("Update pool: tx context error {0}")]
    TxSigningError(TxSigningError),
    #[error("Update pool: stage error {0}")]
    StageError(StageError),
    #[error("Update pool: node error {0}")]
    Node(NodeError),
    #[error("No change address in node")]
    NoChangeAddressSetInNode,
    #[error("Update pool: address encoder error: {0}")]
    AddressEncoderError(AddressEncoderError),
}

pub fn update_pool(
    new_pool_box_hash_str: Option<String>,
    reward_token_id: Option<String>,
    reward_token_amount: Option<u64>,
    height: Option<u64>,
) -> Result<(), UpdatePoolError> {
    let wallet = crate::wallet::WalletData {};
    let op = OraclePool::new().unwrap();
    let change_address_str = get_wallet_status()?
        .change_address
        .ok_or(UpdatePoolError::NoChangeAddressSetInNode)?;

    let network_prefix = if ORACLE_CONFIG.on_mainnet {
        NetworkPrefix::Mainnet
    } else {
        NetworkPrefix::Testnet
    };
    let change_address =
        AddressEncoder::new(network_prefix).parse_address_from_str(&change_address_str)?;

    let new_pool_contract = PoolContract::new()
        .with_update_nft_token_id(ORACLE_CONFIG.update_nft.clone())
        .with_refresh_nft_token_id(ORACLE_CONFIG.refresh_nft.clone());
    let new_pool_box_hash = blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    );

    display_update_diff(
        op.get_pool_box_source().get_pool_box()?,
        &new_pool_contract,
        None,
    );
    if new_pool_box_hash_str.is_none() {
        println!(
            "Run ./oracle-core --new_pool_box_hash {} --height HEIGHT to update pool",
            String::from(new_pool_box_hash)
        );
        return Ok(());
    }
    let height = height.unwrap();
    let new_reward_tokens = reward_token_id
        .zip(reward_token_amount)
        .map(|(token_id, amount)| Token {
            token_id: TokenId::from_base64(&token_id).unwrap(),
            amount: amount.try_into().unwrap(),
        });
    if height != current_block_height()? {
        println!("Height outdated, please use current blockchain height");
        std::process::exit(exitcode::SOFTWARE);
    }
    let tx = build_update_pool_box_tx(
        op.get_pool_box_source(),
        op.get_ballot_boxes_source(),
        &wallet,
        op.get_update_box_source(),
        new_pool_contract,
        new_reward_tokens,
        current_block_height()? as u32,
        change_address,
    )?;
    println!("{}", serde_json::to_string(&tx.spending_tx).unwrap());

    let tx_id_str = sign_and_submit_transaction(&tx.spending_tx)?;
    println!(
        "Update pool box transaction submitted: view here, {}",
        ergo_explorer_transaction_link(tx_id_str, network_prefix)
    );
    Ok(())
}

fn display_update_diff(
    old_pool_box: PoolBoxWrapper,
    new_pool_contract: &PoolContract,
    new_tokens: Option<Token>,
) {
    let new_tokens = new_tokens.unwrap_or(old_pool_box.reward_token().clone());
    println!("Pool Box Parameters: ");
    let pool_box_hash = blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    );
    println!("Pool Box Hash (new): {}", String::from(pool_box_hash));
    println!(
        "Reward Token ID (old): {}",
        String::from(old_pool_box.reward_token().token_id.clone())
    );
    println!(
        "Reward Token ID (new): {}",
        String::from(new_tokens.token_id.clone())
    );
    println!(
        "Reward Token Amount (old): {}",
        old_pool_box.reward_token().amount.as_u64()
    );
    println!("Reward Token Amount (new): {}", new_tokens.amount.as_u64());
    println!(
        "Update NFT ID (old): {}",
        String::from(old_pool_box.contract().update_nft_token_id().clone())
    );
    println!(
        "Update NFT ID (new): {}",
        String::from(new_pool_contract.update_nft_token_id().clone())
    );
    println!(
        "Refresh NFT ID (old): {}",
        String::from(old_pool_box.contract().refresh_nft_token_id().clone())
    );
    println!(
        "Refresh NFT ID (new): {}",
        String::from(new_pool_contract.refresh_nft_token_id().clone())
    );
}

fn build_update_pool_box_tx(
    pool_box_source: &dyn PoolBoxSource,
    ballot_boxes: &dyn BallotBoxesSource,
    wallet: &dyn WalletDataSource,
    update_box: &dyn UpdateBoxSource,
    new_pool_contract: PoolContract,
    new_reward_tokens: Option<Token>,
    height: u32,
    change_address: Address,
) -> Result<TransactionContext<UnsignedTransaction>, UpdatePoolError> {
    let min_votes = update_box.get_update_box()?.min_votes();
    let old_pool_box = pool_box_source.get_pool_box()?;
    let update_box = update_box.get_update_box()?;
    let pool_box_hash = Constant::from(blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    ));
    let reward_tokens = new_reward_tokens.unwrap_or(old_pool_box.reward_token());
    // Find ballot boxes that are voting for the new pool hash
    let mut sorted_ballot_boxes = ballot_boxes.get_ballot_boxes()?;
    sorted_ballot_boxes.sort_by_key(|ballot_box| ballot_box.ballot_token().amount);

    let mut votes_cast = 0;
    let vote_ballot_boxes: Vec<BallotBoxWrapper> = ballot_boxes
        .get_ballot_boxes()?
        .into_iter()
        .filter(|ballot_box| {
            let ballot_box = ballot_box.get_box();
            ballot_box
                .additional_registers
                .get(NonMandatoryRegisterId::R5)
                == Some(&update_box.get_box().creation_info().0.into())
                && ballot_box
                    .additional_registers
                    .get(NonMandatoryRegisterId::R6)
                    == Some(&pool_box_hash)
                && ballot_box
                    .additional_registers
                    .get(NonMandatoryRegisterId::R7)
                    == Some(&reward_tokens.token_id.clone().into())
                && ballot_box
                    .additional_registers
                    .get(NonMandatoryRegisterId::R8)
                    == Some(&(*reward_tokens.amount.as_u64() as i64).into())
        })
        .scan(&mut votes_cast, |votes_cast, ballot_box| {
            if **votes_cast >= min_votes as u64 {
                return None;
            }
            **votes_cast += *ballot_box.ballot_token().amount.as_u64();
            Some(ballot_box)
        })
        .collect();
    if votes_cast < min_votes as u64 {
        return Err(UpdatePoolError::NotEnoughVotes(
            min_votes as usize,
            vote_ballot_boxes.len(),
        ));
    }

    let pool_box_candidate = make_pool_box_candidate(
        &new_pool_contract,
        old_pool_box.rate() as i64,
        old_pool_box.epoch_counter() as i32,
        old_pool_box.pool_nft_token(),
        reward_tokens.clone(),
        old_pool_box.get_box().value,
        old_pool_box.get_box().creation_height, // creation info must be preserved
    )?;
    let mut update_box_candidate =
        ErgoBoxCandidateBuilder::new(update_box.get_box().value, update_box.ergo_tree(), height);
    update_box_candidate.add_token(update_box.update_nft());
    let update_box_candidate = update_box_candidate.build()?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let target_balance = BoxValue::SAFE_USER_MIN;

    let tokens_needed; // Amount of tokens we need from wallet box for new pool box
    let target_tokens: &[Token] = if reward_tokens.token_id != old_pool_box.reward_token().token_id
    {
        tokens_needed = [reward_tokens.clone()];
        &tokens_needed
    } else if reward_tokens.amount > old_pool_box.reward_token().amount {
        let diff = reward_tokens
            .amount
            .checked_sub(&old_pool_box.reward_token().amount)
            .unwrap();
        tokens_needed = [Token {
            token_id: reward_tokens.token_id,
            amount: diff,
        }];
        &tokens_needed
    } else {
        &[]
    };
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, target_balance, target_tokens)?;
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
        box_selection.clone(),
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
    let unsigned_tx = tx_builder.build()?;
    Ok(TransactionContext::new(
        unsigned_tx,
        box_selection.boxes.into(),
        vec![],
    )?)
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
        let new_reward_tokens = Token {
            token_id: force_any_tokenid(),
            amount: force_any_val(),
        };
        let update_contract = UpdateContract::new()
            .with_min_votes(4)
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
                new_reward_tokens.clone(),
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
            // create a wallet box with new reward tokens
            secret.public_image(),
            BoxValue::SAFE_USER_MIN
                .checked_mul_u32(4_000_000_000)
                .unwrap(),
            Some(vec![new_reward_tokens.clone()].try_into().unwrap()),
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
            Some(new_reward_tokens),
            height + 1,
            change_address,
        )
        .unwrap();

        wallet.sign_transaction(update_tx, &ctx, None).unwrap();
    }
}
