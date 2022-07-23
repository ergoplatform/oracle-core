use std::convert::{TryFrom, TryInto};

use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilderError,
        transaction::unsigned::UnsignedTransaction,
    },
    ergo_chain_types::{Digest32, DigestNError},
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{
        address::{Address, AddressEncoder, AddressEncoderError, NetworkPrefix},
        ergo_box::box_value::BoxValue,
        token::{Token, TokenAmount, TokenId},
    },
    wallet::{
        box_selector::{BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;

use crate::{
    box_kind::{make_local_ballot_box_candidate, BallotBox},
    cli_commands::ergo_explorer_transaction_link,
    contracts::ballot::{
        BallotContract, BallotContractError, BallotContractInputs, BallotContractParameters,
    },
    node_interface::{current_block_height, get_wallet_status, sign_and_submit_transaction},
    oracle_config::{TokenIds, ORACLE_CONFIG},
    oracle_state::{LocalBallotBoxSource, OraclePool, StageError},
    wallet::WalletDataSource,
};
use derive_more::From;
use thiserror::Error;

#[derive(Debug, Error, From)]
pub enum VoteUpdatePoolError {
    #[error("Vote update pool: stage error {0}")]
    StageError(StageError),
    #[error("Vote update pool: ErgoBoxCandidateBuilder error {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("Vote update pool: node error {0}")]
    Node(NodeError),
    #[error("Vote update pool: box selector error {0}")]
    BoxSelector(BoxSelectorError),
    #[error("Vote update pool: tx builder error {0}")]
    TxBuilder(TxBuilderError),
    #[error("Vote update pool: Node doesn't have a change address set")]
    NoChangeAddressSetInNode,
    #[error("Vote update pool: AddressEncoder error: {0}")]
    AddressEncoder(AddressEncoderError),
    #[error("Vote update pool: Ballot token owner address not P2PK")]
    IncorrectBallotTokenOwnerAddress,
    #[error("Vote update pool: IO error {0}")]
    Io(std::io::Error),
    #[error("Vote update pool: Digest32 error {0}")]
    Digest(DigestNError),
    #[error("Vote update pool: Ballot contract error {0}")]
    BallotContract(BallotContractError),
}

pub fn vote_update_pool(
    wallet: &dyn WalletDataSource,
    new_pool_box_address_hash_str: String,
    reward_token_id_str: String,
    reward_token_amount: u32,
    update_box_creation_height: u32,
) -> Result<(), VoteUpdatePoolError> {
    let op = OraclePool::new().unwrap();
    let change_address_str = get_wallet_status()?
        .change_address
        .ok_or(VoteUpdatePoolError::NoChangeAddressSetInNode)?;

    let network_prefix = if ORACLE_CONFIG.on_mainnet {
        NetworkPrefix::Mainnet
    } else {
        NetworkPrefix::Testnet
    };
    let change_address =
        AddressEncoder::new(network_prefix).parse_address_from_str(&change_address_str)?;
    let height = current_block_height()? as u32;
    let new_pool_box_address_hash = Digest32::try_from(new_pool_box_address_hash_str)?;
    let reward_token_id = TokenId::from_base64(&reward_token_id_str)?;
    let unsigned_tx = if let Some(local_ballot_box_source) = op.get_local_ballot_box_source() {
        // Note: the ballot box contains the ballot token, but the box is guarded by the contract,
        // which stipulates that the address in R4 is the 'owner' of the token
        build_tx_with_existing_ballot_box(
            local_ballot_box_source,
            wallet,
            new_pool_box_address_hash.clone(),
            reward_token_id.clone(),
            reward_token_amount,
            update_box_creation_height,
            height,
            change_address,
        )?
    } else {
        // Ballot token is assumed to be in some unspent box of the node's wallet.
        build_tx_for_first_ballot_box(
            wallet,
            new_pool_box_address_hash.clone(),
            reward_token_id.clone(),
            reward_token_amount,
            update_box_creation_height,
            AddressEncoder::new(network_prefix).parse_address_from_str(
                &ORACLE_CONFIG.ballot_parameters.ballot_token_owner_address,
            )?,
            &ORACLE_CONFIG.ballot_parameters.contract_parameters,
            &ORACLE_CONFIG.token_ids,
            height,
            change_address,
        )?
    };
    println!(
        "YOU WILL BE CASTING A VOTE FOR THE FOLLOWING ITEMS:\
           - Hash of new pool box address: {}\
           - Reward token Id: {}\
           - Reward token amount: {}\
         TYPE 'YES' TO INITIATE THE TRANSACTION.
        ",
        String::from(new_pool_box_address_hash),
        String::from(reward_token_id),
        reward_token_amount,
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input == "YES" {
        let tx_id_str = sign_and_submit_transaction(&unsigned_tx)?;
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
    local_ballot_box_source: &dyn LocalBallotBoxSource,
    wallet: &dyn WalletDataSource,
    new_pool_box_address_hash: Digest32,
    reward_token_id: TokenId,
    reward_token_amount: u32,
    update_box_creation_height: u32,
    height: u32,
    change_address: Address,
) -> Result<UnsignedTransaction, VoteUpdatePoolError> {
    let in_ballot_box = local_ballot_box_source.get_ballot_box()?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let target_balance = BoxValue::try_from(in_ballot_box.min_storage_rent()).unwrap();
    let reward_token = Token {
        token_id: reward_token_id,
        amount: TokenAmount::try_from(reward_token_amount as u64).unwrap(),
    };
    let ballot_box_candidate = make_local_ballot_box_candidate(
        in_ballot_box.contract(),
        in_ballot_box.ballot_token_owner(),
        update_box_creation_height,
        in_ballot_box.ballot_token(),
        new_pool_box_address_hash,
        reward_token,
        target_balance,
        update_box_creation_height,
    )?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, target_balance, &[])?;
    let mut input_boxes = vec![in_ballot_box.get_box().clone()];
    input_boxes.append(selection.boxes.as_vec().clone().as_mut());
    let box_selection = BoxSelection {
        boxes: input_boxes.try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };
    let mut tx_builder = TxBuilder::new(
        box_selection,
        vec![ballot_box_candidate],
        height,
        BoxValue::SAFE_USER_MIN,
        change_address,
        BoxValue::MIN,
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
    reward_token_id: TokenId,
    reward_token_amount: u32,
    update_box_creation_height: u32,
    ballot_token_owner_address: Address,
    ballot_contract_parameters: &BallotContractParameters,
    token_ids: &TokenIds,
    height: u32,
    change_address: Address,
) -> Result<UnsignedTransaction, VoteUpdatePoolError> {
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let target_balance = BoxValue::try_from(ballot_contract_parameters.min_storage_rent).unwrap();
    let reward_token = Token {
        token_id: reward_token_id,
        amount: TokenAmount::try_from(reward_token_amount as u64).unwrap(),
    };
    let inputs = BallotContractInputs {
        contract_parameters: ballot_contract_parameters,
        update_nft_token_id: &token_ids.update_nft_token_id,
    };
    let contract = BallotContract::new(inputs)?;
    let ballot_token = Token {
        token_id: token_ids.ballot_token_id.clone(),
        amount: 1.try_into().unwrap(),
    };
    if let Address::P2Pk(ballot_token_owner) = &ballot_token_owner_address {
        let ballot_box_candidate = make_local_ballot_box_candidate(
            &contract,
            ballot_token_owner.clone(),
            update_box_creation_height,
            ballot_token.clone(),
            new_pool_box_address_hash,
            reward_token,
            target_balance,
            height,
        )?;
        let box_selector = SimpleBoxSelector::new();
        let selection_target_balance = target_balance
            .checked_add(&BoxValue::SAFE_USER_MIN)
            .unwrap();
        let selection =
            box_selector.select(unspent_boxes, selection_target_balance, &[ballot_token])?;
        let box_selection = BoxSelection {
            boxes: selection.boxes.as_vec().clone().try_into().unwrap(),
            change_boxes: selection.change_boxes,
        };
        let mut tx_builder = TxBuilder::new(
            box_selection,
            vec![ballot_box_candidate],
            height,
            BoxValue::SAFE_USER_MIN,
            change_address,
            BoxValue::MIN,
        );
        // The following context value ensures that `outIndex` in the ballot contract is properly set.
        let ctx_ext = ContextExtension {
            values: vec![(0, 0i32.into())].into_iter().collect(),
        };
        tx_builder.set_context_extension(selection.boxes.first().box_id(), ctx_ext);
        let tx = tx_builder.build()?;
        Ok(tx)
    } else {
        Err(VoteUpdatePoolError::IncorrectBallotTokenOwnerAddress)
    }
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
        contracts::ballot::{BallotContract, BallotContractParameters},
        oracle_config::{BallotBoxWrapperParameters, CastBallotBoxVoteParameters},
        pool_commands::test_utils::{
            find_input_boxes, generate_token_ids, make_wallet_unspent_box, BallotBoxMock,
            WalletDataMock,
        },
        wallet::WalletDataSource,
    };

    use super::{build_tx_for_first_ballot_box, build_tx_with_existing_ballot_box};

    #[test]
    fn test_vote_update_pool_no_existing_ballot_box() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;

        let secret = force_any_val::<DlogProverInput>();
        let new_pool_box_address_hash = force_any_val::<Digest32>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let network_prefix = ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet;
        let change_address = AddressEncoder::new(network_prefix)
            .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
            .unwrap();

        let token_ids = generate_token_ids();
        let ballot_contract_parameters = BallotContractParameters::default();
        let ballot_token = Token {
            token_id: token_ids.ballot_token_id.clone(),
            amount: 1.try_into().unwrap(),
        };
        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BoxValue::SAFE_USER_MIN
                .checked_mul_u32(100_000_000)
                .unwrap(),
            Some(BoxTokens::from_vec(vec![ballot_token]).unwrap()),
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };

        let new_reward_token_id = force_any_val::<TokenId>();
        let unsigned_tx = build_tx_for_first_ballot_box(
            &wallet_mock,
            new_pool_box_address_hash,
            new_reward_token_id,
            100_000,
            height - 3,
            AddressEncoder::new(network_prefix)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap(),
            &ballot_contract_parameters,
            &token_ids,
            height,
            change_address,
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
        let height = ctx.pre_header.height;

        let secret = force_any_val::<DlogProverInput>();
        let new_pool_box_address_hash = force_any_val::<Digest32>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let network_prefix = ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet;
        let change_address = AddressEncoder::new(network_prefix)
            .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
            .unwrap();

        let ballot_contract_parameters = BallotContractParameters::default();
        let token_ids = generate_token_ids();
        let ballot_token = Token {
            token_id: token_ids.ballot_token_id.clone(),
            amount: 1.try_into().unwrap(),
        };
        let ballot_token_owner_address = AddressEncoder::encode_address_as_string(
            network_prefix,
            &Address::P2Pk(secret.public_image()),
        );
        let wrapper_parameters = BallotBoxWrapperParameters {
            contract_parameters: ballot_contract_parameters.clone(),
            ballot_token_owner_address,
            vote_parameters: Some(CastBallotBoxVoteParameters {
                reward_token_id: force_any_val::<TokenId>(),
                reward_token_quantity: 100000,
                pool_box_address_hash: force_any_val::<Digest32>().into(),
            }),
        };
        let inputs = BallotBoxWrapperInputs {
            parameters: &wrapper_parameters,
            ballot_token_id: &token_ids.ballot_token_id,
            update_nft_token_id: &token_ids.update_nft_token_id,
        };
        let in_ballot_box = ErgoBox::from_box_candidate(
            &make_local_ballot_box_candidate(
                &BallotContract::new(inputs.into()).unwrap(),
                secret.public_image(),
                height - 2,
                ballot_token,
                new_pool_box_address_hash.clone(),
                Token {
                    token_id: token_ids.reward_token_id.clone(),
                    amount: 100_000.try_into().unwrap(),
                },
                BoxValue::new(10_000_000).unwrap(),
                height - 2,
            )
            .unwrap(),
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();
        let ballot_box_mock = BallotBoxMock {
            ballot_box: BallotBoxWrapper::new(in_ballot_box.clone(), inputs).unwrap(),
        };
        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BoxValue::SAFE_USER_MIN
                .checked_mul_u32(100_000_000)
                .unwrap(),
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };
        let unsigned_tx = build_tx_with_existing_ballot_box(
            &ballot_box_mock,
            &wallet_mock,
            new_pool_box_address_hash,
            token_ids.reward_token_id,
            100_000,
            height - 3,
            height,
            change_address,
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
