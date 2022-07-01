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
    contracts::ballot::BallotContract,
    node_interface::{current_block_height, get_wallet_status, sign_and_submit_transaction},
    oracle_config::ORACLE_CONFIG,
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

    let prefix = if ORACLE_CONFIG.on_mainnet {
        NetworkPrefix::Mainnet
    } else {
        NetworkPrefix::Testnet
    };
    let change_address = AddressEncoder::new(prefix).parse_address_from_str(&change_address_str)?;
    let height = current_block_height()? as u32;
    let new_pool_box_address_hash = Digest32::try_from(new_pool_box_address_hash_str)?;
    let reward_token_id = TokenId::from_base64(&reward_token_id_str)?;
    let unsigned_tx = if let Some(local_ballot_box_source) = op.get_local_ballot_box_source() {
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
        // ballot token is assumed to be in some unspent box of the node's wallet.

        // note: the ballot box contains the ballot token, but the box is guarded by the contract,
        // which stipulates that the address in R4 is the 'owner' of the token

        build_tx_for_first_ballot_box(
            wallet,
            new_pool_box_address_hash.clone(),
            reward_token_id.clone(),
            reward_token_amount,
            update_box_creation_height,
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
            ergo_explorer_transaction_link(tx_id_str, prefix)
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
        target_balance,
        change_address,
        BoxValue::MIN,
    );
    // The following context value ensures that `outIndex` in the oracle contract is properly set.
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
    height: u32,
    change_address: Address,
) -> Result<UnsignedTransaction, VoteUpdatePoolError> {
    let min_storage_rent = ORACLE_CONFIG.ballot_box_min_storage_rent;
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let target_balance = BoxValue::try_from(min_storage_rent).unwrap();
    let reward_token = Token {
        token_id: reward_token_id,
        amount: TokenAmount::try_from(reward_token_amount as u64).unwrap(),
    };
    let contract = BallotContract::new()
        .with_min_storage_rent(min_storage_rent)
        .with_update_nft_token_id(ORACLE_CONFIG.update_nft.clone());
    let prefix = if ORACLE_CONFIG.on_mainnet {
        NetworkPrefix::Mainnet
    } else {
        NetworkPrefix::Testnet
    };
    let ballot_token_owner = AddressEncoder::new(prefix)
        .parse_address_from_str(&ORACLE_CONFIG.ballot_token_owner_address)?;
    let ballot_token = Token {
        token_id: ORACLE_CONFIG.ballot_token_id.clone(),
        amount: 1.try_into().unwrap(),
    };
    if let Address::P2Pk(ballot_token_owner) = &ballot_token_owner {
        let ballot_box_candidate = make_local_ballot_box_candidate(
            &contract,
            ballot_token_owner.clone(),
            update_box_creation_height,
            ballot_token,
            new_pool_box_address_hash,
            reward_token,
            target_balance,
            height,
        )?;
        let box_selector = SimpleBoxSelector::new();
        let selection = box_selector.select(unspent_boxes, target_balance, &[])?;
        let box_selection = BoxSelection {
            boxes: selection.boxes.as_vec().clone().try_into().unwrap(),
            change_boxes: selection.change_boxes,
        };
        let mut tx_builder = TxBuilder::new(
            box_selection,
            vec![ballot_box_candidate],
            height,
            target_balance,
            change_address,
            BoxValue::MIN,
        );
        // The following context value ensures that `outIndex` in the oracle contract is properly set.
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
