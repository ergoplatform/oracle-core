use std::convert::TryInto;

use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::wallet::box_selector::{BoxSelector, SimpleBoxSelector};
use ergo_lib::wallet::signing::{TransactionContext, TxSigningError};
use ergo_lib::{
    chain::ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoder, AddressEncoderError},
            token::Token,
        },
        serialization::SigmaParsingError,
    },
    wallet::{
        box_selector::{BoxSelection, BoxSelectorError},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::node_interface::node_api::NodeApiTrait;
use crate::oracle_config::ORACLE_CONFIG;
use crate::{
    box_kind::{
        make_collected_oracle_box_candidate, make_oracle_box_candidate, OracleBox, OracleBoxWrapper,
    },
    explorer_api::ergo_explorer_transaction_link,
    oracle_config::BASE_FEE,
    oracle_state::{DataSourceError, LocalDatapointBoxSource},
    oracle_types::BlockHeight,
    spec_token::SpecToken,
};

#[derive(Debug, Error)]
pub enum ExtractRewardTokensActionError {
    #[error("Oracle box must contain at least 2 reward tokens. It contains {0} tokens")]
    InsufficientRewardTokensInOracleBox(usize),
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
    #[error("tx signing error: {0}")]
    TxSigningError(#[from] TxSigningError),
    #[error("Sigma parsing error: {0}")]
    SigmaParse(#[from] SigmaParsingError),
    #[error("tx builder error: {0}")]
    TxBuilder(#[from] TxBuilderError),
    #[error("No local datapoint box")]
    NoLocalDatapointBox,
    #[error("AddressEncoder error: {0}")]
    AddressEncoder(#[from] AddressEncoderError),
    #[error("Node doesn't have a change address set")]
    NoChangeAddressSetInNode,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn extract_reward_tokens(
    node_api: &dyn NodeApiTrait,
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    rewards_destination_str: String,
    height: BlockHeight,
) -> Result<(), anyhow::Error> {
    let rewards_destination =
        AddressEncoder::unchecked_parse_network_address_from_str(&rewards_destination_str)?;
    let network_prefix = rewards_destination.network();
    let oracle_address = ORACLE_CONFIG.oracle_address.clone();
    let change_address = ORACLE_CONFIG.change_address.clone();
    let (context, num_reward_tokens) = build_extract_reward_tokens_tx(
        local_datapoint_box_source,
        node_api,
        rewards_destination.address(),
        height,
        oracle_address,
        change_address.unwrap().address(),
    )?;

    println!(
        "YOU WILL BE TRANSFERRING {} REWARD TOKENS TO {}. TYPE 'YES' TO INITIATE THE TRANSACTION.",
        num_reward_tokens, rewards_destination_str
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() == "YES" {
        let signed_tx = node_api.sign_transaction(context)?;
        let tx_id = node_api.submit_transaction(&signed_tx)?;
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

fn build_extract_reward_tokens_tx(
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    node_api: &dyn NodeApiTrait,
    rewards_destination: Address,
    height: BlockHeight,
    oracle_address: NetworkAddress,
    change_address: Address,
) -> Result<(TransactionContext<UnsignedTransaction>, u64), ExtractRewardTokensActionError> {
    let in_oracle_box = local_datapoint_box_source
        .get_local_oracle_datapoint_box()?
        .ok_or(ExtractRewardTokensActionError::NoLocalDatapointBox)?;
    let num_reward_tokens = *in_oracle_box.reward_token().amount.as_u64();
    if num_reward_tokens <= 1 {
        return Err(
            ExtractRewardTokensActionError::InsufficientRewardTokensInOracleBox(
                num_reward_tokens as usize,
            ),
        );
    }
    if let Address::P2Pk(_) = &rewards_destination {
        let single_reward_token = SpecToken {
            token_id: in_oracle_box.reward_token().token_id,
            amount: 1.try_into().unwrap(),
        };
        let oracle_box_candidate =
            if let OracleBoxWrapper::Posted(ref posted_oracle_box) = in_oracle_box {
                make_oracle_box_candidate(
                    posted_oracle_box.contract(),
                    posted_oracle_box.public_key(),
                    posted_oracle_box.rate(),
                    posted_oracle_box.epoch_counter(),
                    posted_oracle_box.oracle_token(),
                    single_reward_token,
                    posted_oracle_box.get_box().value,
                    height,
                )?
            } else {
                make_collected_oracle_box_candidate(
                    in_oracle_box.contract(),
                    in_oracle_box.public_key(),
                    in_oracle_box.oracle_token(),
                    single_reward_token,
                    in_oracle_box.get_box().value,
                    height,
                )?
            };

        // Build box to hold extracted tokens
        let mut builder =
            ErgoBoxCandidateBuilder::new(*BASE_FEE, rewards_destination.script()?, height.0);

        let extracted_reward_tokens = Token {
            token_id: in_oracle_box.reward_token().token_id(),
            amount: (num_reward_tokens - 1).try_into().unwrap(),
        };

        builder.add_token(extracted_reward_tokens);
        let reward_box_candidate = builder.build()?;

        // `BASE_FEE` each for the fee and the box holding the extracted reward tokens.
        let target_balance = BASE_FEE.checked_mul_u32(2).unwrap();
        let unspent_boxes = node_api.get_unspent_boxes_by_address(
            &oracle_address.to_base58(),
            target_balance,
            [].into(),
        )?;

        let box_selector = SimpleBoxSelector::new();
        let selection = box_selector.select(unspent_boxes, target_balance, &[])?;
        let mut input_boxes = vec![in_oracle_box.get_box().clone()];
        input_boxes.append(selection.boxes.as_vec().clone().as_mut());
        let box_selection = BoxSelection {
            boxes: input_boxes.try_into().unwrap(),
            change_boxes: selection.change_boxes,
        };
        let inputs = box_selection.boxes.clone().to_vec();
        let mut tx_builder = TxBuilder::new(
            box_selection,
            vec![oracle_box_candidate, reward_box_candidate],
            height.0,
            *BASE_FEE,
            change_address,
        );
        // The following context value ensures that `outIndex` in the oracle contract is properly set.
        let ctx_ext = ContextExtension {
            values: vec![(0, 0i32.into())].into_iter().collect(),
        };
        tx_builder.set_context_extension(in_oracle_box.get_box().box_id(), ctx_ext);
        let tx = tx_builder.build()?;
        let context = match TransactionContext::new(tx, inputs, vec![]) {
            Ok(ctx) => ctx,
            Err(e) => return Err(ExtractRewardTokensActionError::TxSigningError(e)),
        };
        Ok((context, num_reward_tokens - 1))
    } else {
        Err(ExtractRewardTokensActionError::IncorrectDestinationAddress)
    }
}

#[cfg(test)]
mod tests {

    use std::convert::TryFrom;

    use super::*;
    use crate::box_kind::{OracleBoxWrapper, OracleBoxWrapperInputs};
    use crate::contracts::oracle::OracleContractParameters;
    use crate::node_interface::test_utils::{MockNodeApi, SubmitTxMock};
    use crate::oracle_types::EpochCounter;
    use crate::pool_commands::test_utils::{
        generate_token_ids, make_datapoint_box, make_wallet_unspent_box, OracleBoxMock,
    };
    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use sigma_test_util::force_any_val;

    #[test]
    fn test_extract_reward_tokens() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let secret = force_any_val::<DlogProverInput>();
        let oracle_pub_key = secret.public_image().h;

        let num_reward_tokens_in_box = 2;

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
                BlockHeight(height.0),
                num_reward_tokens_in_box,
            ),
            &oracle_box_wrapper_inputs,
        )
        .unwrap();
        let local_datapoint_box_source = OracleBoxMock { oracle_box };

        let address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10000).unwrap(),
            None,
        );
        let mock_node_api = &MockNodeApi {
            unspent_boxes: vec![wallet_unspent_box],
            ctx: ctx.clone(),
            secrets: vec![secret.clone().into()],
            submitted_txs: &SubmitTxMock::default().transactions,
            chain_submit_tx: None,
        };
        let (tx_context, num_reward_tokens) = build_extract_reward_tokens_tx(
            &local_datapoint_box_source,
            mock_node_api,
            address.address(),
            height,
            address.clone(),
            address.address(),
        )
        .unwrap();

        assert_eq!(num_reward_tokens, num_reward_tokens_in_box - 1);

        let _signed_tx = mock_node_api.sign_transaction(tx_context).unwrap();
    }
}
