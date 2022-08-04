use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilder,
        ergo_box::box_builder::ErgoBoxCandidateBuilderError,
        transaction::unsigned::UnsignedTransaction,
    },
    ergo_chain_types::blake2b256_hash,
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{
        address::{Address, AddressEncoder, AddressEncoderError, NetworkPrefix},
        ergo_box::{box_value::BoxValue, ErgoBox, NonMandatoryRegisterId},
        token::{Token, TokenId},
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
use log::{error, info};
use std::convert::TryInto;

use crate::{
    box_kind::{make_pool_box_candidate, BallotBox, BallotBoxWrapper, PoolBox, PoolBoxWrapper},
    cli_commands::ergo_explorer_transaction_link,
    contracts::pool::PoolContract,
    contracts::pool::PoolContractInputs,
    node_interface::{current_block_height, get_wallet_status, sign_and_submit_transaction},
    oracle_config::{OracleConfig, ORACLE_CONFIG},
    oracle_state::{BallotBoxesSource, OraclePool, PoolBoxSource, StageError, UpdateBoxSource},
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
    #[error("Update pool: address encoder error {0}")]
    AddressEncoderError(AddressEncoderError),
    #[error("Update pool: pool contract error {0}")]
    PoolContractError(crate::contracts::pool::PoolContractError),
    #[error("Update pool: io error {0}")]
    IoError(std::io::Error),
    #[error("Update pool: yaml error {0}")]
    YamlError(serde_yaml::Error),
    #[error("Update pool: could not find unspent wallot boxes that do not contain ballot tokens")]
    NoUsableWalletBoxes,
}

pub fn update_pool(
    new_pool_box_hash_str: Option<String>,
    reward_token_id: Option<String>,
    reward_token_amount: Option<u64>,
    height: Option<u64>,
) -> Result<(), UpdatePoolError> {
    info!("Opening oracle_config_updated.yaml");
    let s = std::fs::read_to_string("oracle_config_updated.yaml")?;
    let new_oracle_config: OracleConfig = serde_yaml::from_str(&s)?;
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

    let pool_contract_inputs = PoolContractInputs::from((
        &new_oracle_config.pool_contract_parameters,
        &new_oracle_config.token_ids,
    ));

    let new_pool_contract = PoolContract::new(pool_contract_inputs)?;
    let new_pool_box_hash = blake2b256_hash(
        &new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap(),
    );

    display_update_diff(
        &ORACLE_CONFIG,
        &new_oracle_config,
        op.get_pool_box_source().get_pool_box()?,
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

    let tx_id_str = sign_and_submit_transaction(&tx.spending_tx)?;
    println!(
        "Update pool box transaction submitted: view here, {}",
        ergo_explorer_transaction_link(tx_id_str, network_prefix)
    );
    Ok(())
}

fn display_update_diff(
    old_oracle_config: &OracleConfig,
    new_oracle_config: &OracleConfig,
    old_pool_box: PoolBoxWrapper,
    new_reward_tokens: Option<Token>,
) {
    let new_tokens = new_reward_tokens.unwrap_or(old_pool_box.reward_token());
    let new_pool_contract = PoolContract::new(PoolContractInputs::from((
        &new_oracle_config.pool_contract_parameters,
        &new_oracle_config.token_ids,
    )))
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
        String::from(old_oracle_config.token_ids.reward_token_id.clone())
    );
    println!(
        "Reward Token ID (new): {}",
        String::from(new_oracle_config.token_ids.reward_token_id.clone())
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
        error!("Could not find unspent wallet boxes that do not contain ballot token");
        return Err(UpdatePoolError::NoUsableWalletBoxes);
    }

    let target_balance = BoxValue::SAFE_USER_MIN;
    let tokens_needed; // Amount of reward tokens we need from wallet box for new pool box
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
        outputs.push(ballot_box_candidate.build()?)
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
        ergo_chain_types::blake2b256_hash,
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::{
            chain::{
                address::AddressEncoder,
                ergo_box::{box_value::BoxValue, ErgoBox},
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
            make_local_ballot_box_candidate, make_pool_box_candidate, BallotBoxWrapper,
            PoolBoxWrapper, PoolBoxWrapperInputs, UpdateBoxWrapper, UpdateBoxWrapperInputs,
        },
        contracts::{
            ballot::{BallotContract, BallotContractInputs},
            pool::{PoolContract, PoolContractInputs},
            update::{UpdateContract, UpdateContractInputs, UpdateContractParameters},
        },
        oracle_config::{BallotBoxWrapperParameters, TokenIds},
        pool_commands::test_utils::{
            make_wallet_unspent_box, BallotBoxesMock, PoolBoxMock, UpdateBoxMock, WalletDataMock,
        },
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

        let token_ids = TokenIds {
            pool_nft_token_id: force_any_tokenid(),
            update_nft_token_id: force_any_tokenid(),
            refresh_nft_token_id: force_any_tokenid(),
            reward_token_id: force_any_tokenid(),
            oracle_token_id: force_any_tokenid(),
            ballot_token_id: force_any_tokenid(),
        };
        let reward_tokens = Token {
            token_id: token_ids.reward_token_id.clone(),
            amount: 1500.try_into().unwrap(),
        };
        let new_reward_tokens = Token {
            token_id: force_any_tokenid(),
            amount: force_any_val(),
        };

        let update_contract_parameters = UpdateContractParameters {
            min_votes: 6,
            ..Default::default()
        };
        let update_contract_inputs = UpdateContractInputs {
            contract_parameters: &update_contract_parameters,
            pool_nft_token_id: &token_ids.pool_nft_token_id,
            ballot_token_id: &token_ids.ballot_token_id,
        };
        let update_contract = UpdateContract::new(update_contract_inputs).unwrap();
        let mut update_box_candidate = ErgoBoxCandidateBuilder::new(
            BoxValue::SAFE_USER_MIN,
            update_contract.ergo_tree(),
            height,
        );
        update_box_candidate.add_token(Token {
            token_id: token_ids.update_nft_token_id.clone(),
            amount: 1.try_into().unwrap(),
        });
        let update_box = ErgoBox::from_box_candidate(
            &update_box_candidate.build().unwrap(),
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();

        let pool_contract_parameters = Default::default();
        let pool_contract_inputs =
            PoolContractInputs::from((&pool_contract_parameters, &token_ids));

        let pool_contract = PoolContract::new(pool_contract_inputs).unwrap();
        let pool_box_candidate = make_pool_box_candidate(
            &pool_contract,
            0,
            0,
            Token {
                token_id: token_ids.pool_nft_token_id.clone(),
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
        let mut new_pool_contract_inputs = pool_contract_inputs.clone();
        new_pool_contract_inputs.refresh_nft_token_id = &new_refresh_token_id;
        let new_pool_contract = PoolContract::new(new_pool_contract_inputs).unwrap();

        let pool_box_bytes = new_pool_contract
            .ergo_tree()
            .sigma_serialize_bytes()
            .unwrap();
        let pool_box_hash = blake2b256_hash(&pool_box_bytes);

        let ballot_contract_parameters = Default::default();
        let ballot_contract_inputs = BallotContractInputs {
            contract_parameters: &ballot_contract_parameters,
            update_nft_token_id: &token_ids.update_nft_token_id,
        };
        let ballot_contract = BallotContract::new(ballot_contract_inputs).unwrap();

        let mut ballot_boxes = vec![];

        for _ in 0..6 {
            let secret = DlogProverInput::random();
            let ballot_box_parameters = BallotBoxWrapperParameters {
                contract_parameters: ballot_contract_parameters.clone(),
                vote_parameters: Some(crate::oracle_config::CastBallotBoxVoteParameters {
                    pool_box_address_hash: base16::encode_lower(&pool_box_hash),
                    reward_token_id: new_reward_tokens.token_id.clone(),
                    reward_token_quantity: *new_reward_tokens.amount.as_u64() as u32,
                }),
                ballot_token_owner_address: AddressEncoder::new(
                    ballot_contract_parameters.p2s.network(),
                )
                .address_to_str(
                    &ergo_lib::ergotree_ir::chain::address::Address::P2Pk(secret.public_image()),
                ),
            };
            let ballot_box_candidate = make_local_ballot_box_candidate(
                &ballot_contract,
                secret.public_image(),
                update_box.creation_height,
                Token {
                    token_id: token_ids.ballot_token_id.clone(),
                    amount: 1.try_into().unwrap(),
                },
                pool_box_hash.clone(),
                new_reward_tokens.clone(),
                ballot_contract.min_storage_rent().try_into().unwrap(),
                height,
            )
            .unwrap();
            let ballot_box =
                ErgoBox::from_box_candidate(&ballot_box_candidate, force_any_val::<TxId>(), 0)
                    .unwrap();
            ballot_boxes.push(
                BallotBoxWrapper::new(
                    ballot_box,
                    crate::box_kind::BallotBoxWrapperInputs {
                        parameters: &ballot_box_parameters,
                        ballot_token_id: &token_ids.ballot_token_id,
                        update_nft_token_id: &token_ids.update_nft_token_id,
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
            update_box: UpdateBoxWrapper::new(
                update_box,
                UpdateBoxWrapperInputs {
                    contract_parameters: &update_contract_parameters,
                    update_nft_token_id: &token_ids.update_nft_token_id,
                    ballot_token_id: &token_ids.ballot_token_id,
                    pool_nft_token_id: &token_ids.pool_nft_token_id,
                },
            )
            .unwrap(),
        };
        let pool_mock = PoolBoxMock {
            pool_box: PoolBoxWrapper::new(
                pool_box,
                PoolBoxWrapperInputs {
                    contract_parameters: &pool_contract_parameters,
                    pool_nft_token_id: &token_ids.pool_nft_token_id,
                    reward_token_id: &token_ids.reward_token_id,
                    refresh_nft_token_id: &token_ids.refresh_nft_token_id,
                    update_nft_token_id: &token_ids.update_nft_token_id,
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
        println!("{}", serde_json::to_string(&update_tx.spending_tx).unwrap());

        wallet.sign_transaction(update_tx, &ctx, None).unwrap();
    }
}
