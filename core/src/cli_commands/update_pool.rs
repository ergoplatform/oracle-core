use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilder,
        ergo_box::box_builder::ErgoBoxCandidateBuilderError,
        transaction::unsigned::UnsignedTransaction,
    },
    ergo_chain_types::blake2b256_hash,
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{
        address::Address,
        ergo_box::{ErgoBox, NonMandatoryRegisterId},
        token::Token,
    },
    ergotree_ir::serialization::SigmaSerializable,
    wallet::{
        box_selector::{BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector},
        signing::{TransactionContext, TxSigningError},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use log::{error, info};
use std::convert::TryInto;

use crate::{
    box_kind::{
        make_pool_box_candidate_unchecked, BallotBox, PoolBox, PoolBoxWrapper, VoteBallotBoxWrapper,
    },
    cli_commands::ergo_explorer_transaction_link,
    contracts::pool::PoolContract,
    node_interface::{SignTransaction, SubmitTransaction},
    oracle_config::BASE_FEE,
    oracle_state::{OraclePool, PoolBoxSource, StageError, UpdateBoxSource, VoteBallotBoxesSource},
    pool_config::{CastBallotBoxVoteParameters, PoolConfig, POOL_CONFIG},
    spec_token::TokenIdKind,
    wallet::{WalletDataError, WalletDataSource},
};
use derive_more::From;
use thiserror::Error;

#[derive(Debug, Error, From)]
pub enum UpdatePoolError {
    #[error("Update pool: Not enough votes for {2:?}, expected {0}, found {1}")]
    NotEnoughVotes(usize, usize, CastBallotBoxVoteParameters),
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
    #[error("Update pool: pool contract error {0}")]
    PoolContractError(crate::contracts::pool::PoolContractError),
    #[error("Update pool: io error {0}")]
    IoError(std::io::Error),
    #[error("Update pool: yaml error {0}")]
    YamlError(serde_yaml::Error),
    #[error("Update pool: could not find unspent wallot boxes that do not contain ballot tokens")]
    NoUsableWalletBoxes,
    #[error("WalletData error: {0}")]
    WalletData(WalletDataError),
}

pub fn update_pool(
    op: &OraclePool,
    wallet: &dyn WalletDataSource,
    tx_signer: &dyn SignTransaction,
    tx_submit: &dyn SubmitTransaction,
    new_pool_box_hash_str: Option<String>,
    new_reward_tokens: Option<Token>,
    height: u32,
) -> Result<(), UpdatePoolError> {
    info!("Opening pool_config_updated.yaml");
    let s = std::fs::read_to_string("pool_config_updated.yaml")?;
    let new_pool_config: PoolConfig = serde_yaml::from_str(&s)?;
    let (change_address, network_prefix) = {
        let net_addr = wallet.get_change_address()?;
        (net_addr.address(), net_addr.network())
    };

    let new_pool_contract =
        PoolContract::checked_load(&new_pool_config.pool_box_wrapper_inputs.contract_inputs)?;
    let new_pool_box_hash = blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    );

    display_update_diff(
        &POOL_CONFIG,
        &new_pool_config,
        op.get_pool_box_source().get_pool_box()?,
        new_reward_tokens.clone(),
    );

    if new_pool_box_hash_str.is_none() {
        println!(
            "Run ./oracle-core --new_pool_box_hash {} to update pool",
            String::from(new_pool_box_hash)
        );
        return Ok(());
    }

    let tx = build_update_pool_box_tx(
        op.get_pool_box_source(),
        op.get_ballot_boxes_source(),
        wallet,
        op.get_update_box_source(),
        new_pool_contract,
        new_reward_tokens,
        height,
        change_address,
    )?;

    let signed_tx = tx_signer.sign_transaction(&tx.spending_tx)?;
    let tx_id_str = tx_submit.submit_transaction(&signed_tx)?;
    println!(
        "Update pool box transaction submitted: view here, {}",
        ergo_explorer_transaction_link(tx_id_str, network_prefix)
    );
    Ok(())
}

fn display_update_diff(
    old_pool_config: &PoolConfig,
    new_pool_config: &PoolConfig,
    old_pool_box: PoolBoxWrapper,
    new_reward_tokens: Option<Token>,
) {
    let new_tokens = new_reward_tokens.unwrap_or_else(|| old_pool_box.reward_token().into());
    let new_pool_contract =
        PoolContract::checked_load(&new_pool_config.pool_box_wrapper_inputs.contract_inputs)
            .unwrap();
    println!("Pool Parameters: ");
    let pool_box_hash = blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    );
    println!("Pool Box Hash (new): {}", String::from(pool_box_hash));
    println!(
        "Reward Token ID (old): {}",
        String::from(old_pool_config.token_ids.reward_token_id.token_id())
    );
    println!(
        "Reward Token ID (new): {}",
        String::from(new_pool_config.token_ids.reward_token_id.token_id())
    );
    println!(
        "Reward Token Amount (old): {}",
        old_pool_box.reward_token().amount.as_u64()
    );
    println!("Reward Token Amount (new): {}", new_tokens.amount.as_u64());
    println!(
        "Update NFT ID (old): {}",
        String::from(old_pool_box.contract().update_nft_token_id())
    );
    println!(
        "Update NFT ID (new): {}",
        String::from(new_pool_contract.update_nft_token_id())
    );
    println!(
        "Refresh NFT ID (old): {}",
        String::from(old_pool_box.contract().refresh_nft_token_id())
    );
    println!(
        "Refresh NFT ID (new): {}",
        String::from(new_pool_contract.refresh_nft_token_id())
    );
}

#[allow(clippy::too_many_arguments)]
fn build_update_pool_box_tx(
    pool_box_source: &dyn PoolBoxSource,
    ballot_boxes: &dyn VoteBallotBoxesSource,
    wallet: &dyn WalletDataSource,
    update_box: &dyn UpdateBoxSource,
    new_pool_contract: PoolContract,
    new_reward_tokens: Option<Token>,
    height: u32,
    change_address: Address,
) -> Result<TransactionContext<UnsignedTransaction>, UpdatePoolError> {
    let update_box = update_box.get_update_box()?;
    let min_votes = update_box.min_votes();
    let old_pool_box = pool_box_source.get_pool_box()?;
    let pool_box_hash = blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    );
    let reward_tokens = new_reward_tokens.unwrap_or_else(|| old_pool_box.reward_token().into());
    let vote_parameters = CastBallotBoxVoteParameters {
        pool_box_address_hash: pool_box_hash,
        reward_token_id: reward_tokens.token_id,
        reward_token_quantity: *reward_tokens.amount.as_u64(),
        update_box_creation_height: update_box.get_box().creation_height as i32,
    };
    // Find ballot boxes that are voting for the new pool hash
    let mut sorted_ballot_boxes = ballot_boxes.get_ballot_boxes()?;
    // Sort in descending order of ballot token amounts. If two boxes have the same amount of ballot tokens, also compare box value, in case some boxes were incorrectly created below minStorageRent
    sorted_ballot_boxes.sort_by(|b1, b2| {
        (
            *b1.ballot_token().amount.as_u64(),
            *b1.get_box().value.as_u64(),
        )
            .cmp(&(
                *b2.ballot_token().amount.as_u64(),
                *b2.get_box().value.as_u64(),
            ))
    });
    sorted_ballot_boxes.reverse();

    let mut votes_cast = 0;
    let vote_ballot_boxes: Vec<VoteBallotBoxWrapper> = ballot_boxes
        .get_ballot_boxes()?
        .into_iter()
        .filter(|ballot_box| *ballot_box.vote_parameters() == vote_parameters)
        .scan(&mut votes_cast, |votes_cast, ballot_box| {
            **votes_cast += *ballot_box.ballot_token().amount.as_u64();
            Some(ballot_box)
        })
        .collect();
    if votes_cast < min_votes as u64 {
        return Err(UpdatePoolError::NotEnoughVotes(
            min_votes as usize,
            vote_ballot_boxes.len(),
            vote_parameters,
        ));
    }

    let pool_box_candidate = make_pool_box_candidate_unchecked(
        &new_pool_contract,
        old_pool_box.rate(),
        old_pool_box.epoch_counter() as i32,
        old_pool_box.pool_nft_token(),
        reward_tokens.clone(),
        old_pool_box.get_box().value,
        height,
    )?;
    let mut update_box_candidate =
        ErgoBoxCandidateBuilder::new(update_box.get_box().value, update_box.ergo_tree(), height);
    update_box_candidate.add_token(update_box.update_nft());
    let update_box_candidate = update_box_candidate.build()?;

    // Find unspent boxes without ballot token, see: https://github.com/ergoplatform/oracle-core/pull/80#issuecomment-1200258458
    let unspent_boxes: Vec<ErgoBox> = wallet
        .get_unspent_wallet_boxes()?
        .into_iter()
        .filter(|wallet_box| {
            wallet_box
                .tokens
                .as_ref()
                .and_then(|tokens| {
                    tokens
                        .iter()
                        .find(|token| token.token_id == update_box.ballot_token_id())
                })
                .is_none()
        })
        .collect();
    if unspent_boxes.is_empty() {
        error!("Could not find unspent wallet boxes that do not contain ballot token. Please move ballot tokens to another address");
        return Err(UpdatePoolError::NoUsableWalletBoxes);
    }

    let target_balance = *BASE_FEE;
    let target_tokens = if reward_tokens.token_id != old_pool_box.reward_token().token_id() {
        vec![reward_tokens.clone()]
    } else {
        vec![]
    };
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, target_balance, &target_tokens)?;
    let mut input_boxes = vec![old_pool_box.get_box().clone(), update_box.get_box().clone()];
    input_boxes.extend(
        vote_ballot_boxes
            .iter()
            .map(|ballot_box| ballot_box.get_box())
            .cloned(),
    );
    input_boxes.extend_from_slice(selection.boxes.as_vec());
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
        ballot_box_candidate.add_token(ballot_box.ballot_token().into());
        ballot_box_candidate.set_register_value(
            NonMandatoryRegisterId::R4,
            (*ballot_box.ballot_token_owner().h).clone().into(),
        );
        outputs.push(ballot_box_candidate.build()?)
    }

    let mut tx_builder = TxBuilder::new(
        box_selection.clone(),
        outputs.clone(),
        height,
        *BASE_FEE,
        change_address,
    );

    if reward_tokens.token_id != old_pool_box.reward_token().token_id() {
        tx_builder.set_token_burn_permit(vec![old_pool_box.reward_token().into()]);
    }

    for (i, input_ballot) in vote_ballot_boxes.iter().enumerate() {
        tx_builder.set_context_extension(
            input_ballot.get_box().box_id(),
            ContextExtension {
                values: IntoIterator::into_iter([(0, ((i + 2) as i32).into())]).collect(), // first 2 outputs are pool and update box, ballot indexes start at 2
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
        ergo_chain_types::blake2b256_hash,
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::{
            chain::{
                address::AddressEncoder,
                ergo_box::ErgoBox,
                token::{Token, TokenId},
            },
            serialization::SigmaSerializable,
        },
        wallet::Wallet,
    };
    use sigma_test_util::force_any_val;
    use std::convert::TryInto;

    use crate::{
        box_kind::{
            make_local_ballot_box_candidate, make_pool_box_candidate, PoolBoxWrapper,
            PoolBoxWrapperInputs, UpdateBoxWrapper, UpdateBoxWrapperInputs, VoteBallotBoxWrapper,
        },
        contracts::{
            ballot::{BallotContract, BallotContractInputs, BallotContractParameters},
            pool::{PoolContract, PoolContractInputs},
            update::{UpdateContract, UpdateContractInputs, UpdateContractParameters},
        },
        oracle_config::BASE_FEE,
        pool_commands::test_utils::{
            generate_token_ids, make_wallet_unspent_box, BallotBoxesMock, PoolBoxMock,
            UpdateBoxMock, WalletDataMock,
        },
        spec_token::{RefreshTokenId, SpecToken, TokenIdKind},
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

        let token_ids = generate_token_ids();
        dbg!(&token_ids);
        let reward_tokens = SpecToken {
            token_id: token_ids.reward_token_id.clone(),
            amount: 1500.try_into().unwrap(),
        };
        let new_reward_tokens = Token {
            token_id: force_any_tokenid(),
            amount: force_any_val(),
        };
        dbg!(&new_reward_tokens);

        let default_update_contract_parameters = UpdateContractParameters::default();
        let update_contract_parameters = UpdateContractParameters::build_with(
            default_update_contract_parameters.ergo_tree_bytes(),
            default_update_contract_parameters.pool_nft_index(),
            default_update_contract_parameters.ballot_token_index(),
            default_update_contract_parameters.min_votes_index(),
            6,
        )
        .unwrap();
        let update_contract_inputs = UpdateContractInputs::build_with(
            update_contract_parameters,
            token_ids.pool_nft_token_id.clone(),
            token_ids.ballot_token_id.clone(),
        )
        .unwrap();
        let update_contract = UpdateContract::checked_load(&update_contract_inputs).unwrap();
        let mut update_box_candidate =
            ErgoBoxCandidateBuilder::new(*BASE_FEE, update_contract.ergo_tree(), height);
        update_box_candidate.add_token(Token {
            token_id: token_ids.update_nft_token_id.token_id(),
            amount: 1.try_into().unwrap(),
        });
        let update_box = ErgoBox::from_box_candidate(
            &update_box_candidate.build().unwrap(),
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();

        let pool_contract_parameters = Default::default();
        let pool_contract_inputs = PoolContractInputs::build_with(
            pool_contract_parameters,
            token_ids.refresh_nft_token_id,
            token_ids.update_nft_token_id.clone(),
        )
        .unwrap();

        let pool_contract = PoolContract::build_with(&pool_contract_inputs).unwrap();
        let pool_box_candidate = make_pool_box_candidate(
            &pool_contract,
            0,
            0,
            SpecToken {
                token_id: token_ids.pool_nft_token_id.clone(),
                amount: 1.try_into().unwrap(),
            },
            reward_tokens.clone(),
            *BASE_FEE,
            height,
        )
        .unwrap();
        let pool_box =
            ErgoBox::from_box_candidate(&pool_box_candidate, force_any_val::<TxId>(), 0).unwrap();

        let new_refresh_token_id = force_any_tokenid();
        let mut new_pool_contract_inputs = pool_contract_inputs.clone();
        new_pool_contract_inputs.refresh_nft_token_id =
            RefreshTokenId::from_token_id_unchecked(new_refresh_token_id);
        let new_pool_contract = PoolContract::build_with(&new_pool_contract_inputs).unwrap();

        let pool_box_bytes = new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap();
        let pool_box_hash = blake2b256_hash(&pool_box_bytes);

        let ballot_contract_parameters = BallotContractParameters::default();
        let ballot_contract_inputs = BallotContractInputs::build_with(
            ballot_contract_parameters.clone(),
            token_ids.update_nft_token_id.clone(),
        )
        .unwrap();
        let ballot_contract = BallotContract::checked_load(&ballot_contract_inputs).unwrap();

        let mut ballot_boxes = vec![];

        for _ in 0..6 {
            let secret = DlogProverInput::random();
            let ballot_box_candidate = make_local_ballot_box_candidate(
                &ballot_contract,
                secret.public_image(),
                update_box.creation_height,
                SpecToken {
                    token_id: token_ids.ballot_token_id.clone(),
                    amount: 1.try_into().unwrap(),
                },
                pool_box_hash,
                new_reward_tokens.clone(),
                ballot_contract.min_storage_rent(),
                height,
            )
            .unwrap();
            let ballot_box =
                ErgoBox::from_box_candidate(&ballot_box_candidate, force_any_val::<TxId>(), 0)
                    .unwrap();
            ballot_boxes.push(
                VoteBallotBoxWrapper::new(
                    ballot_box,
                    &crate::box_kind::BallotBoxWrapperInputs {
                        ballot_token_id: token_ids.ballot_token_id.clone(),
                        contract_inputs: ballot_contract_inputs.clone(),
                    },
                )
                .unwrap(),
            );
        }
        let ballot_boxes_mock = BallotBoxesMock { ballot_boxes };

        let secret = DlogProverInput::random();
        let wallet_unspent_box = make_wallet_unspent_box(
            // create a wallet box with new reward tokens
            secret.public_image(),
            BASE_FEE.checked_mul_u32(4_000_000_000).unwrap(),
            Some(vec![new_reward_tokens.clone()].try_into().unwrap()),
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let update_mock = UpdateBoxMock {
            update_box: UpdateBoxWrapper::new(
                update_box,
                &UpdateBoxWrapperInputs {
                    contract_inputs: update_contract_inputs.clone(),
                    update_nft_token_id: token_ids.update_nft_token_id,
                },
            )
            .unwrap(),
        };
        let pool_mock = PoolBoxMock {
            pool_box: PoolBoxWrapper::new(
                pool_box,
                &PoolBoxWrapperInputs {
                    contract_inputs: pool_contract_inputs,
                    pool_nft_token_id: token_ids.pool_nft_token_id,
                    reward_token_id: token_ids.reward_token_id,
                },
            )
            .unwrap(),
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
