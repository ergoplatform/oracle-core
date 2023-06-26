use std::convert::{TryFrom, TryInto};

use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilderError,
        transaction::unsigned::UnsignedTransaction,
    },
    ergo_chain_types::{Digest32, DigestNError, EcPoint},
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::address::Address,
    wallet::{
        box_selector::{BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;

use crate::{
    box_kind::{make_local_ballot_box_candidate, BallotBox, BallotBoxWrapper},
    contracts::ballot::{
        BallotContract, BallotContractError, BallotContractInputs, BallotContractParameters,
    },
    explorer_api::ergo_explorer_transaction_link,
    node_interface::{SignTransaction, SubmitTransaction},
    oracle_config::{BASE_FEE, ORACLE_CONFIG},
    oracle_state::{DataSourceError, LocalBallotBoxSource},
    oracle_types::BlockHeight,
    pool_config::{TokenIds, POOL_CONFIG},
    spec_token::{RewardTokenId, SpecToken, TokenIdKind},
    wallet::{WalletDataError, WalletDataSource},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoteUpdatePoolError {
    #[error("Vote update pool: data source error {0}")]
    DataSourceError(#[from] DataSourceError),
    #[error("Vote update pool: ErgoBoxCandidateBuilder error {0}")]
    ErgoBoxCandidateBuilder(#[from] ErgoBoxCandidateBuilderError),
    #[error("Vote update pool: node error {0}")]
    Node(#[from] NodeError),
    #[error("Vote update pool: box selector error {0}")]
    BoxSelector(#[from] BoxSelectorError),
    #[error("Vote update pool: tx builder error {0}")]
    TxBuilder(#[from] TxBuilderError),
    #[error("Vote update pool: Node doesn't have a change address set")]
    NoChangeAddressSetInNode,
    #[error("Vote update pool: Ballot token owner address not P2PK")]
    IncorrectBallotTokenOwnerAddress,
    #[error("Vote update pool: IO error {0}")]
    Io(#[from] std::io::Error),
    #[error("Vote update pool: Digest32 error {0}")]
    Digest(#[from] DigestNError),
    #[error("Vote update pool: Ballot contract error {0}")]
    BallotContract(#[from] BallotContractError),
    #[error("WalletData error: {0}")]
    WalletData(#[from] WalletDataError),
}

#[allow(clippy::too_many_arguments)]
pub fn vote_update_pool(
    wallet: &dyn WalletDataSource,
    tx_signer: &dyn SignTransaction,
    tx_submit: &dyn SubmitTransaction,
    local_ballot_box_source: &dyn LocalBallotBoxSource,
    new_pool_box_address_hash_str: String,
    reward_token_opt: Option<SpecToken<RewardTokenId>>,
    update_box_creation_height: BlockHeight,
    height: BlockHeight,
    ballot_contract: &BallotContract,
) -> Result<(), anyhow::Error> {
    let change_network_address = wallet.get_change_address()?;
    let network_prefix = change_network_address.network();
    let new_pool_box_address_hash = Digest32::try_from(new_pool_box_address_hash_str)?;
    let ballot_token_owner =
        if let Address::P2Pk(ballot_token_owner) = ORACLE_CONFIG.oracle_address.address() {
            ballot_token_owner.h
        } else {
            return Err(VoteUpdatePoolError::IncorrectBallotTokenOwnerAddress.into());
        };
    let unsigned_tx = if let Some(local_ballot_box) = local_ballot_box_source.get_ballot_box()? {
        log::debug!("Found local ballot box");
        // Note: the ballot box contains the ballot token, but the box is guarded by the contract,
        // which stipulates that the address in R4 is the 'owner' of the token
        build_tx_with_existing_ballot_box(
            &local_ballot_box,
            ballot_contract,
            wallet,
            new_pool_box_address_hash,
            reward_token_opt.clone(),
            update_box_creation_height,
            height,
            change_network_address.address(),
            ballot_token_owner.as_ref(),
        )?
    } else {
        log::debug!("Not found local ballot box, looking for a ballot token in the wallet");
        // Note: the ballot box contains the ballot token, but the box is guarded by the contract,
        // Ballot token is assumed to be in some unspent box of the node's wallet.
        build_tx_for_first_ballot_box(
            wallet,
            new_pool_box_address_hash,
            reward_token_opt.clone(),
            update_box_creation_height,
            ballot_token_owner.as_ref(),
            POOL_CONFIG
                .ballot_box_wrapper_inputs
                .contract_inputs
                .contract_parameters(),
            &POOL_CONFIG.token_ids,
            height,
            change_network_address.address(),
        )?
    };
    println!(
        "YOU WILL BE CASTING A VOTE FOR THE FOLLOWING ITEMS:\
           - Hash of new pool box contract: {}",
        String::from(new_pool_box_address_hash),
    );
    if let Some(reward_token) = reward_token_opt {
        println!(
            "  - Reward token Id: {}\
               - Reward token amount: {}",
            String::from(reward_token.token_id.token_id()),
            reward_token.amount.as_u64(),
        );
    }
    println!("TYPE 'YES' TO INITIATE THE TRANSACTION.");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim_end() == "YES" {
        log::debug!(
            "Signing vote tx: {:?} ",
            &serde_json::to_string(&unsigned_tx)
        );
        let signed_tx = tx_signer.sign_transaction(&unsigned_tx)?;
        log::debug!(
            "Submitting signed vote tx: {:?} ",
            &serde_json::to_string(&signed_tx)
        );
        let tx_id_str = tx_submit.submit_transaction(&signed_tx)?;
        crate::explorer_api::wait_for_tx_confirmation(signed_tx.id());
        println!(
            "Transaction made. Check status here: {}",
            ergo_explorer_transaction_link(tx_id_str, network_prefix)
        );
    } else {
        println!("Aborting the transaction.")
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_tx_with_existing_ballot_box(
    in_ballot_box: &BallotBoxWrapper,
    ballot_contract: &BallotContract,
    wallet: &dyn WalletDataSource,
    new_pool_box_address_hash: Digest32,
    reward_token_opt: Option<SpecToken<RewardTokenId>>,
    update_box_creation_height: BlockHeight,
    height: BlockHeight,
    change_address: Address,
    ballot_token_owner_pk: &EcPoint,
) -> Result<UnsignedTransaction, VoteUpdatePoolError> {
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    #[allow(clippy::todo)]
    let ballot_box_candidate = make_local_ballot_box_candidate(
        ballot_contract,
        ballot_token_owner_pk,
        update_box_creation_height,
        in_ballot_box.ballot_token(),
        new_pool_box_address_hash,
        reward_token_opt,
        in_ballot_box.get_box().value,
        height,
    )?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, *BASE_FEE, &[])?;
    let mut input_boxes = vec![in_ballot_box.get_box().clone()];
    input_boxes.append(selection.boxes.as_vec().clone().as_mut());
    let box_selection = BoxSelection {
        boxes: input_boxes.try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };
    let mut tx_builder = TxBuilder::new(
        box_selection,
        vec![ballot_box_candidate],
        height.0,
        *BASE_FEE,
        change_address,
    );
    // The following context value ensures that `outIndex` in the ballot contract is properly set.
    let ctx_ext = ContextExtension {
        values: vec![(0, 0i32.into())].into_iter().collect(),
    };
    tx_builder.set_context_extension(in_ballot_box.get_box().box_id(), ctx_ext);
    let tx = tx_builder.build()?;
    Ok(tx)
}

#[allow(clippy::too_many_arguments)]
fn build_tx_for_first_ballot_box(
    wallet: &dyn WalletDataSource,
    new_pool_box_address_hash: Digest32,
    reward_token_opt: Option<SpecToken<RewardTokenId>>,
    update_box_creation_height: BlockHeight,
    ballot_token_owner: &EcPoint,
    ballot_contract_parameters: &BallotContractParameters,
    token_ids: &TokenIds,
    height: BlockHeight,
    change_address: Address,
) -> Result<UnsignedTransaction, VoteUpdatePoolError> {
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let out_ballot_box_value = ballot_contract_parameters.min_storage_rent();
    let inputs = BallotContractInputs::build_with(
        ballot_contract_parameters.clone(),
        token_ids.update_nft_token_id.clone(),
    )?;
    let contract = BallotContract::checked_load(&inputs)?;
    let ballot_token = SpecToken {
        token_id: token_ids.ballot_token_id.clone(),
        amount: 1.try_into().unwrap(),
    };
    let ballot_box_candidate = make_local_ballot_box_candidate(
        &contract,
        ballot_token_owner,
        update_box_creation_height,
        ballot_token.clone(),
        new_pool_box_address_hash,
        reward_token_opt,
        out_ballot_box_value,
        height,
    )?;
    let box_selector = SimpleBoxSelector::new();
    let selection_target_balance = out_ballot_box_value.checked_add(&BASE_FEE).unwrap();
    let selection = box_selector.select(
        unspent_boxes,
        selection_target_balance,
        &[ballot_token.into()],
    )?;
    let box_selection = BoxSelection {
        boxes: selection.boxes.as_vec().clone().try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };
    let mut tx_builder = TxBuilder::new(
        box_selection,
        vec![ballot_box_candidate],
        height.0,
        *BASE_FEE,
        change_address,
    );
    // The following context value ensures that `outIndex` in the ballot contract is properly set.
    let ctx_ext = ContextExtension {
        values: vec![(0, 0i32.into())].into_iter().collect(),
    };
    tx_builder.set_context_extension(selection.boxes.first().box_id(), ctx_ext);
    let tx = tx_builder.build()?;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::{
        chain::{ergo_state_context::ErgoStateContext, transaction::TxId},
        ergo_chain_types::Digest32,
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::{Address, AddressEncoder},
            ergo_box::{box_value::BoxValue, BoxTokens, ErgoBox},
            token::{Token, TokenId},
        },
        wallet::{signing::TransactionContext, Wallet},
    };
    use sigma_test_util::force_any_val;

    use crate::{
        box_kind::{make_local_ballot_box_candidate, BallotBoxWrapper, BallotBoxWrapperInputs},
        contracts::ballot::{BallotContract, BallotContractInputs, BallotContractParameters},
        oracle_config::BASE_FEE,
        oracle_types::{BlockHeight, EpochLength},
        pool_commands::test_utils::{
            find_input_boxes, generate_token_ids, make_wallet_unspent_box, WalletDataMock,
        },
        spec_token::{RewardTokenId, SpecToken, TokenIdKind},
        wallet::WalletDataSource,
    };

    use super::{build_tx_for_first_ballot_box, build_tx_with_existing_ballot_box};

    #[test]
    fn test_vote_update_pool_no_existing_ballot_box() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);

        let secret = force_any_val::<DlogProverInput>();
        let new_pool_box_address_hash = force_any_val::<Digest32>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let token_ids = generate_token_ids();
        let ballot_contract_inputs = BallotContractInputs::build_with(
            BallotContractParameters::default(),
            token_ids.update_nft_token_id.clone(),
        )
        .unwrap();

        let ballot_token = Token {
            token_id: token_ids.ballot_token_id.token_id(),
            amount: 1.try_into().unwrap(),
        };
        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(100_000_000).unwrap(),
            Some(BoxTokens::from_vec(vec![ballot_token]).unwrap()),
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
            change_address: change_address.clone(),
        };

        let new_reward_token = SpecToken {
            token_id: RewardTokenId::from_token_id_unchecked(force_any_val::<TokenId>()),
            amount: 100_000.try_into().unwrap(),
        };

        let ballot_token_owner = if let Address::P2Pk(ballot_token_owner) = change_address.address()
        {
            ballot_token_owner.h
        } else {
            panic!("Expected P2PK address");
        };
        let unsigned_tx = build_tx_for_first_ballot_box(
            &wallet_mock,
            new_pool_box_address_hash,
            Some(new_reward_token),
            BlockHeight(height.0) - 3,
            &ballot_token_owner,
            ballot_contract_inputs.contract_parameters(),
            &token_ids,
            height,
            change_address.address(),
        )
        .unwrap();

        let tx_context = TransactionContext::new(
            unsigned_tx.clone(),
            find_input_boxes(unsigned_tx, wallet_mock.get_unspent_wallet_boxes().unwrap()),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }

    #[test]
    fn test_vote_update_pool_with_existing_ballot_box() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);

        let secret = force_any_val::<DlogProverInput>();
        let new_pool_box_address_hash = force_any_val::<Digest32>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let ballot_contract_parameters = BallotContractParameters::default();
        let token_ids = generate_token_ids();
        let ballot_token = SpecToken {
            token_id: token_ids.ballot_token_id.clone(),
            amount: 1.try_into().unwrap(),
        };
        let inputs = BallotBoxWrapperInputs {
            ballot_token_id: token_ids.ballot_token_id.clone(),
            contract_inputs: BallotContractInputs::build_with(
                ballot_contract_parameters.clone(),
                token_ids.update_nft_token_id.clone(),
            )
            .unwrap(),
        };
        let ballot_contract = BallotContract::checked_load(&inputs.contract_inputs).unwrap();
        let in_ballot_box = ErgoBox::from_box_candidate(
            &make_local_ballot_box_candidate(
                &ballot_contract,
                secret.public_image().h.as_ref(),
                height - EpochLength(2),
                ballot_token,
                new_pool_box_address_hash,
                Some(SpecToken {
                    token_id: token_ids.reward_token_id.clone(),
                    amount: 100_000.try_into().unwrap(),
                }),
                BoxValue::new(10_000_000).unwrap(),
                height - EpochLength(2),
            )
            .unwrap(),
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();
        let ballot_box = BallotBoxWrapper::new(in_ballot_box.clone(), &inputs).unwrap();
        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(100_000_000).unwrap(),
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
            change_address: change_address.clone(),
        };
        let unsigned_tx = build_tx_with_existing_ballot_box(
            &ballot_box,
            &ballot_contract,
            &wallet_mock,
            new_pool_box_address_hash,
            Some(SpecToken {
                token_id: token_ids.reward_token_id,
                amount: 100_000.try_into().unwrap(),
            }),
            height - EpochLength(3),
            height,
            change_address.address(),
            secret.public_image().h.as_ref(),
        )
        .unwrap();

        let mut input_boxes = vec![in_ballot_box];
        input_boxes.append(wallet_mock.get_unspent_wallet_boxes().unwrap().as_mut());
        let boxes_to_spend = find_input_boxes(unsigned_tx.clone(), input_boxes);
        assert!(!boxes_to_spend.is_empty());
        let tx_context = TransactionContext::new(unsigned_tx, boxes_to_spend, Vec::new()).unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
