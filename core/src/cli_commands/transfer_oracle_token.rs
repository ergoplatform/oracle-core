use std::convert::TryInto;

use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilderError,
        transaction::unsigned::UnsignedTransaction,
    },
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::{
        chain::address::{Address, AddressEncoder, AddressEncoderError},
        serialization::SigmaParsingError,
    },
    wallet::{
        box_selector::{BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::{
    box_kind::{
        make_collected_oracle_box_candidate, make_oracle_box_candidate, OracleBox, OracleBoxWrapper,
    },
    explorer_api::ergo_explorer_transaction_link,
    node_interface::{SignTransaction, SubmitTransaction},
    oracle_config::BASE_FEE,
    oracle_state::{DataSourceError, LocalDatapointBoxSource},
    oracle_types::BlockHeight,
    wallet::{WalletDataError, WalletDataSource},
};

#[derive(Debug, Error)]
pub enum TransferOracleTokenActionError {
    #[error(
        "Oracle box should contain exactly 1 reward token. It contains {0} tokens. \
        Use `extract-reward-tokens` command to extract reward tokens from the oracle box.`"
    )]
    IncorrectNumberOfRewardTokensInOracleBox(usize),
    #[error("Destination address not P2PK")]
    IncorrectDestinationAddress,
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(#[from] ErgoBoxCandidateBuilderError),
    #[error("data source error: {0}")]
    DataSourceError(#[from] DataSourceError),
    #[error("node error: {0}")]
    Node(#[from] NodeError),
    #[error("box selector error: {0}")]
    BoxSelector(#[from] BoxSelectorError),
    #[error("Sigma parsing error: {0}")]
    SigmaParse(#[from] SigmaParsingError),
    #[error("tx builder error: {0}")]
    TxBuilder(#[from] TxBuilderError),
    #[error("Node doesn't have a change address set")]
    NoChangeAddressSetInNode,
    #[error("No local datapoint box")]
    NoLocalDatapointBox,
    #[error("AddressEncoder error: {0}")]
    AddressEncoder(#[from] AddressEncoderError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("WalletData error: {0}")]
    WalletData(#[from] WalletDataError),
}

pub fn transfer_oracle_token(
    wallet: &dyn WalletDataSource,
    tx_signer: &dyn SignTransaction,
    tx_submit: &dyn SubmitTransaction,
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    rewards_destination_str: String,
    height: BlockHeight,
) -> Result<(), anyhow::Error> {
    let rewards_destination =
        AddressEncoder::unchecked_parse_network_address_from_str(&rewards_destination_str)?;
    let (change_address, network_prefix) = {
        let net_address = wallet.get_change_address()?;
        (net_address.address(), net_address.network())
    };
    let unsigned_tx = build_transfer_oracle_token_tx(
        local_datapoint_box_source,
        wallet,
        rewards_destination.address(),
        height,
        change_address,
    )?;

    println!(
        "YOU WILL BE TRANSFERRING YOUR ORACLE TOKEN TO {}. TYPE 'YES' TO INITIATE THE TRANSACTION.",
        rewards_destination_str
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() == "YES" {
        let signed_tx = tx_signer.sign_transaction(&unsigned_tx)?;
        let tx_id = tx_submit.submit_transaction(&signed_tx)?;
        crate::explorer_api::wait_for_tx_confirmation(signed_tx.id());
        println!(
            "Transaction made. Check status here: {}",
            ergo_explorer_transaction_link(tx_id, network_prefix)
        );
    } else {
        println!("Aborting the transaction.")
    }
    Ok(())
}
fn build_transfer_oracle_token_tx(
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    wallet: &dyn WalletDataSource,
    oracle_token_destination: Address,
    height: BlockHeight,
    change_address: Address,
) -> Result<UnsignedTransaction, TransferOracleTokenActionError> {
    let in_oracle_box = local_datapoint_box_source
        .get_local_oracle_datapoint_box()?
        .ok_or(TransferOracleTokenActionError::NoLocalDatapointBox)?;
    let num_reward_tokens = *in_oracle_box.reward_token().amount.as_u64();
    if num_reward_tokens != 1 {
        return Err(
            TransferOracleTokenActionError::IncorrectNumberOfRewardTokensInOracleBox(
                num_reward_tokens as usize,
            ),
        );
    }
    if let Address::P2Pk(p2pk_dest) = &oracle_token_destination {
        let oracle_box_candidate =
            if let OracleBoxWrapper::Posted(ref posted_oracle_box) = in_oracle_box {
                make_oracle_box_candidate(
                    posted_oracle_box.contract(),
                    *p2pk_dest.h.clone(),
                    posted_oracle_box.rate() as i64,
                    posted_oracle_box.epoch_counter(),
                    posted_oracle_box.oracle_token(),
                    posted_oracle_box.reward_token(),
                    posted_oracle_box.get_box().value,
                    height,
                )?
            } else {
                make_collected_oracle_box_candidate(
                    in_oracle_box.contract(),
                    *p2pk_dest.h.clone(),
                    in_oracle_box.oracle_token(),
                    in_oracle_box.reward_token(),
                    in_oracle_box.get_box().value,
                    height,
                )?
            };

        let unspent_boxes = wallet.get_unspent_wallet_boxes()?;

        let target_balance = *BASE_FEE;

        let box_selector = SimpleBoxSelector::new();
        let selection = box_selector.select(unspent_boxes, target_balance, &[])?;
        let mut input_boxes = vec![in_oracle_box.get_box().clone()];
        input_boxes.append(selection.boxes.as_vec().clone().as_mut());
        let box_selection = BoxSelection {
            boxes: input_boxes.try_into().unwrap(),
            change_boxes: selection.change_boxes,
        };
        let mut tx_builder = TxBuilder::new(
            box_selection,
            vec![oracle_box_candidate],
            height.0,
            target_balance,
            change_address,
        );
        // The following context value ensures that `outIndex` in the oracle contract is properly set.
        let ctx_ext = ContextExtension {
            values: vec![(0, 0i32.into())].into_iter().collect(),
        };
        tx_builder.set_context_extension(in_oracle_box.get_box().box_id(), ctx_ext);
        let tx = tx_builder.build()?;
        Ok(tx)
    } else {
        Err(TransferOracleTokenActionError::IncorrectDestinationAddress)
    }
}

#[cfg(test)]
mod tests {

    use std::convert::TryFrom;

    use super::*;
    use crate::box_kind::{OracleBoxWrapper, OracleBoxWrapperInputs};
    use crate::contracts::oracle::OracleContractParameters;
    use crate::oracle_types::EpochCounter;
    use crate::pool_commands::test_utils::{
        find_input_boxes, generate_token_ids, make_datapoint_box, make_wallet_unspent_box,
        OracleBoxMock, WalletDataMock,
    };
    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use sigma_test_util::force_any_val;

    #[test]
    fn test_transfer_oracle_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let oracle_pub_key = secret.public_image().h;

        let parameters = OracleContractParameters::default();
        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::try_from((parameters, &token_ids)).unwrap();
        let oracle_box = OracleBoxWrapper::new(
            make_datapoint_box(
                *oracle_pub_key,
                200,
                EpochCounter(1),
                &token_ids,
                BASE_FEE.checked_mul_u32(100).unwrap(),
                BlockHeight(height.0) - 9,
                1,
            ),
            &oracle_box_wrapper_inputs,
        )
        .unwrap();
        let local_datapoint_box_source = OracleBoxMock { oracle_box };

        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10000).unwrap(),
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
            change_address: change_address.clone(),
        };
        let tx = build_transfer_oracle_token_tx(
            &local_datapoint_box_source,
            &wallet_mock,
            change_address.address(),
            height,
            change_address.address(),
        )
        .unwrap();

        let mut possible_input_boxes = vec![local_datapoint_box_source
            .get_local_oracle_datapoint_box()
            .unwrap()
            .unwrap()
            .get_box()
            .clone()];
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            tx.clone(),
            find_input_boxes(tx, possible_input_boxes),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
