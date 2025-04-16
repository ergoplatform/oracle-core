use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::wallet::box_selector::{BoxSelector, SimpleBoxSelector};
use ergo_lib::wallet::signing::{TransactionContext, TxSigningError};
use ergo_lib::{
    chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError,
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{address::Address, token::TokenAmount},
    wallet::{
        box_selector::BoxSelectorError,
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use thiserror::Error;

use crate::address_util::address_to_p2pk;
use crate::node_interface::node_api::NodeApiTrait;
use crate::{
    action_report::PublishDatapointActionReport,
    actions::PublishDataPointAction,
    box_kind::{make_oracle_box_candidate, OracleBox, OracleBoxWrapper, OracleBoxWrapperInputs},
    contracts::oracle::{OracleContract, OracleContractError},
    datapoint_source::{DataPointSource, DataPointSourceError},
    oracle_config::BASE_FEE,
    oracle_state::DataSourceError,
    oracle_types::{BlockHeight, EpochCounter},
    spec_token::{OracleTokenId, RewardTokenId, SpecToken},
};

#[derive(Debug, Error)]
pub enum PublishDatapointActionError {
    #[error("data source error: {0}")]
    DataSourceError(#[from] DataSourceError),
    #[error("Oracle box has no reward token")]
    NoRewardTokenInOracleBox,
    #[error("tx builder error: {0}")]
    TxBuilder(#[from] TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(#[from] ErgoBoxCandidateBuilderError),
    #[error("tx signing error: {0}")]
    TxSigningError(#[from] TxSigningError),
    #[error("box selector error: {0}")]
    BoxSelector(#[from] BoxSelectorError),
    #[error("datapoint source error: {0}")]
    DataPointSource(#[from] DataPointSourceError),
    #[error("oracle contract error: {0}")]
    OracleContract(#[from] OracleContractError),
}

#[allow(clippy::too_many_arguments)]
pub fn build_subsequent_publish_datapoint_action(
    local_datapoint_box: &OracleBoxWrapper,
    node_api: &dyn NodeApiTrait,
    height: BlockHeight,
    oracle_address: NetworkAddress,
    change_address: Address,
    datapoint_source: &dyn DataPointSource,
    new_epoch_counter: EpochCounter,
    reward_token_id: &RewardTokenId,
) -> Result<(PublishDataPointAction, PublishDatapointActionReport), PublishDatapointActionError> {
    let new_datapoint = datapoint_source.get_datapoint()?;
    let in_oracle_box = local_datapoint_box;

    let outbox_reward_tokens = if reward_token_id != &in_oracle_box.reward_token().token_id {
        SpecToken {
            token_id: reward_token_id.clone(),
            amount: TokenAmount::try_from(1).unwrap(),
        }
    } else {
        in_oracle_box.reward_token()
    };

    let output_candidate = make_oracle_box_candidate(
        in_oracle_box.contract(),
        in_oracle_box.public_key(),
        new_datapoint,
        new_epoch_counter,
        in_oracle_box.oracle_token(),
        outbox_reward_tokens.clone(),
        in_oracle_box.get_box().value,
        height,
    )?;
    let box_selector = SimpleBoxSelector::new();
    let tx_fee = *BASE_FEE;
    let mut unspent_boxes =
        node_api.get_unspent_boxes_by_address(&oracle_address.to_base58(), tx_fee, vec![])?;
    let target_tokens = vec![
        in_oracle_box.oracle_token().into(),
        outbox_reward_tokens.into(),
    ];
    let target_balance = in_oracle_box.get_box().value.checked_add(&tx_fee).unwrap();
    unspent_boxes.push(in_oracle_box.get_box().clone());
    let selection = box_selector.select(unspent_boxes, target_balance, target_tokens.as_slice())?;
    let inputs = selection.boxes.clone().to_vec();
    let mut tx_builder = TxBuilder::new(
        selection,
        vec![output_candidate],
        height.0,
        tx_fee,
        change_address,
    );

    // The following context value ensures that `outIndex` in the oracle contract is properly set.
    let ctx_ext = ContextExtension {
        values: vec![(0, 0i32.into())].into_iter().collect(),
    };
    tx_builder.set_context_extension(in_oracle_box.get_box().box_id(), ctx_ext);
    let tx = tx_builder.build()?;
    let report = PublishDatapointActionReport {
        posted_datapoint: new_datapoint,
    };
    let context = match TransactionContext::new(tx, inputs, vec![]) {
        Ok(ctx) => ctx,
        Err(e) => return Err(PublishDatapointActionError::TxSigningError(e)),
    };
    Ok((
        PublishDataPointAction {
            transaction_context: context,
        },
        report,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn build_publish_first_datapoint_action(
    node_api: &dyn NodeApiTrait,
    height: BlockHeight,
    oracle_address: NetworkAddress,
    change_address: Address,
    inputs: OracleBoxWrapperInputs,
    datapoint_source: &dyn DataPointSource,
) -> Result<(PublishDataPointAction, PublishDatapointActionReport), PublishDatapointActionError> {
    let new_datapoint = datapoint_source.get_datapoint()?;
    let tx_fee = *BASE_FEE;
    let box_selector = SimpleBoxSelector::new();
    let oracle_token: SpecToken<OracleTokenId> = SpecToken {
        token_id: inputs.oracle_token_id.clone(),
        amount: TokenAmount::try_from(1).unwrap(),
    };
    let reward_token: SpecToken<RewardTokenId> = SpecToken {
        token_id: inputs.reward_token_id.clone(),
        amount: TokenAmount::try_from(1).unwrap(),
    };

    let contract = OracleContract::checked_load(&inputs.contract_inputs)?;
    let min_storage_rent = contract.parameters().min_storage_rent;
    let target_balance = min_storage_rent.checked_add(&tx_fee).unwrap();
    let target_tokens = vec![oracle_token.clone().into(), reward_token.clone().into()];

    let unspent_boxes = node_api.get_unspent_boxes_by_address(
        &oracle_address.to_base58(),
        target_balance,
        target_tokens.clone(),
    )?;
    let box_selection = box_selector.select(
        unspent_boxes.clone(),
        target_balance,
        target_tokens.as_slice(),
    )?;
    let oracle_pk = address_to_p2pk(&oracle_address.address()).unwrap();
    let output_candidate = make_oracle_box_candidate(
        &contract,
        *oracle_pk.h,
        new_datapoint,
        EpochCounter(1),
        oracle_token,
        reward_token,
        min_storage_rent,
        height,
    )?;

    let box_id = box_selection.boxes.first().box_id();
    let inputs = box_selection.boxes.clone().to_vec();
    let mut tx_builder = TxBuilder::new(
        box_selection,
        vec![output_candidate],
        height.0,
        tx_fee,
        change_address,
    );

    // The following context value ensures that `outIndex` in the oracle contract is properly set.
    let ctx_ext = ContextExtension {
        values: vec![(0, 0i32.into())].into_iter().collect(),
    };
    tx_builder.set_context_extension(box_id, ctx_ext);
    let tx = tx_builder.build()?;
    let report = PublishDatapointActionReport {
        posted_datapoint: new_datapoint,
    };
    let context = match TransactionContext::new(tx, inputs, vec![]) {
        Ok(ctx) => ctx,
        Err(e) => return Err(PublishDatapointActionError::TxSigningError(e)),
    };
    Ok((
        PublishDataPointAction {
            transaction_context: context,
        },
        report,
    ))
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use crate::contracts::oracle::OracleContractParameters;
    use crate::node_interface::test_utils::{MockNodeApi, SubmitTxMock};
    use crate::oracle_types::{EpochLength, Rate};
    use crate::pool_commands::test_utils::{
        generate_token_ids, make_datapoint_box, make_wallet_unspent_box,
    };
    use crate::spec_token::TokenIdKind;
    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::{AddressEncoder, NetworkPrefix};
    use ergo_lib::ergotree_ir::chain::ergo_box::{BoxTokens, ErgoBox, NonMandatoryRegisters};
    use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::mir::expr::Expr;
    use sigma_test_util::force_any_val;

    #[derive(Debug)]
    struct MockDatapointSource {
        datapoint: Rate,
    }

    impl DataPointSource for MockDatapointSource {
        fn get_datapoint(&self) -> Result<Rate, DataPointSourceError> {
            Ok(self.datapoint)
        }
    }

    #[test]
    fn test_subsequent_publish_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let oracle_contract_parameters = OracleContractParameters::default();
        let pool_box_epoch_id = EpochCounter(1);
        let secret = force_any_val::<DlogProverInput>();
        let oracle_address = NetworkAddress::new(
            NetworkPrefix::Mainnet,
            &Address::P2Pk(secret.public_image().clone()),
        );
        let oracle_pub_key = secret.public_image().h;
        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::try_from((oracle_contract_parameters, &token_ids)).unwrap();
        let oracle_box = OracleBoxWrapper::new(
            make_datapoint_box(
                *oracle_pub_key,
                200,
                EpochCounter(pool_box_epoch_id.0 - 1),
                &token_ids,
                oracle_box_wrapper_inputs
                    .contract_inputs
                    .contract_parameters()
                    .min_storage_rent,
                height - EpochLength(99),
                100,
            ),
            &oracle_box_wrapper_inputs,
        )
        .unwrap();

        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10000).unwrap(),
            None,
        );
        let mock_node_api = MockNodeApi {
            unspent_boxes: vec![wallet_unspent_box],
            ctx: ctx.clone(),
            secrets: vec![secret.clone().into()],
            submitted_txs: &SubmitTxMock::default().transactions,
            chain_submit_tx: None,
        };

        let datapoint_source = MockDatapointSource {
            datapoint: 201.into(),
        };
        let (action, _) = build_subsequent_publish_datapoint_action(
            &oracle_box,
            &mock_node_api,
            height,
            oracle_address,
            change_address.address(),
            &datapoint_source,
            pool_box_epoch_id,
            &token_ids.reward_token_id,
        )
        .unwrap();

        let _signed_tx = mock_node_api
            .sign_transaction(action.transaction_context)
            .unwrap();
    }

    #[test]
    fn test_first_publish_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);

        let token_ids = generate_token_ids();
        let tokens = BoxTokens::from_vec(vec![
            Token {
                token_id: token_ids.reward_token_id.token_id(),
                amount: 100u64.try_into().unwrap(),
            },
            Token {
                token_id: token_ids.oracle_token_id.token_id(),
                amount: 1u64.try_into().unwrap(),
            },
        ])
        .unwrap();

        let secret = force_any_val::<DlogProverInput>();
        let oracle_address = NetworkAddress::new(
            NetworkPrefix::Mainnet,
            &Address::P2Pk(secret.public_image().clone()),
        );
        let c: Constant = secret.public_image().into();
        let expr: Expr = c.into();
        let ergo_tree = ErgoTree::try_from(expr).unwrap();

        let value = BASE_FEE.checked_mul_u32(10000).unwrap();
        let box_with_tokens = ErgoBox::new(
            value,
            ergo_tree.clone(),
            Some(tokens),
            NonMandatoryRegisters::new(vec![].into_iter().collect()).unwrap(),
            height.0 - 30,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();
        let unspent_boxes = vec![
            box_with_tokens.clone(),
            ErgoBox::new(
                *BASE_FEE,
                ergo_tree.clone(),
                None,
                NonMandatoryRegisters::new(vec![].into_iter().collect()).unwrap(),
                height.0 - 9,
                force_any_val::<TxId>(),
                0,
            )
            .unwrap(),
        ];

        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let oracle_contract_parameters = OracleContractParameters::default();
        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::try_from((oracle_contract_parameters.clone(), &token_ids))
                .unwrap();
        let mock_node_api = MockNodeApi {
            unspent_boxes: unspent_boxes.clone(),
            ctx: ctx.clone(),
            secrets: vec![secret.clone().into()],
            submitted_txs: &SubmitTxMock::default().transactions,
            chain_submit_tx: None,
        };

        let (action, _) = build_publish_first_datapoint_action(
            &mock_node_api,
            height,
            oracle_address,
            change_address.address(),
            oracle_box_wrapper_inputs,
            &MockDatapointSource {
                datapoint: 201.into(),
            },
        )
        .unwrap();

        assert_eq!(
            action
                .transaction_context
                .spending_tx
                .output_candidates
                .first()
                .value,
            oracle_contract_parameters.min_storage_rent
        );

        let _signed_tx = mock_node_api
            .sign_transaction(action.transaction_context)
            .unwrap();
    }

    #[test]
    fn test_subsequent_publish_datapoint_with_minted_reward_token() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let minted_reward_token_id =
            RewardTokenId::from_token_id_unchecked(force_any_val::<TokenId>());
        let oracle_contract_parameters = OracleContractParameters::default();
        let pool_box_epoch_id = EpochCounter(1);
        dbg!(&token_ids);
        dbg!(&minted_reward_token_id);
        let secret = force_any_val::<DlogProverInput>();
        let oracle_address = NetworkAddress::new(
            NetworkPrefix::Mainnet,
            &Address::P2Pk(secret.public_image().clone()),
        );
        let oracle_pub_key = secret.public_image().h;

        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::try_from((oracle_contract_parameters, &token_ids)).unwrap();
        let oracle_box = OracleBoxWrapper::new(
            make_datapoint_box(
                *oracle_pub_key,
                200,
                EpochCounter(pool_box_epoch_id.0 - 1),
                &token_ids,
                oracle_box_wrapper_inputs
                    .contract_inputs
                    .contract_parameters()
                    .min_storage_rent,
                height - EpochLength(99),
                100,
            ),
            &oracle_box_wrapper_inputs,
        )
        .unwrap();

        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10000).unwrap(),
            Some(
                vec![Token {
                    token_id: minted_reward_token_id.token_id(),
                    amount: 1u64.try_into().unwrap(),
                }]
                .try_into()
                .unwrap(),
            ),
        );
        let mock_node_api = MockNodeApi {
            unspent_boxes: vec![wallet_unspent_box],
            ctx: ctx.clone(),
            secrets: vec![secret.clone().into()],
            submitted_txs: &SubmitTxMock::default().transactions,
            chain_submit_tx: None,
        };

        let datapoint_source = MockDatapointSource {
            datapoint: 201.into(),
        };
        let (action, _) = build_subsequent_publish_datapoint_action(
            &oracle_box,
            &mock_node_api,
            height,
            oracle_address,
            change_address.address(),
            &datapoint_source,
            pool_box_epoch_id,
            &minted_reward_token_id,
        )
        .unwrap();

        let _signed_tx = mock_node_api
            .sign_transaction(action.transaction_context)
            .unwrap();
    }
}
