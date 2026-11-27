use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::wallet::box_selector::{BoxSelector, SimpleBoxSelector};
use ergo_lib::wallet::signing::{TransactionContext, TxSigningError};
use ergo_lib::{
    chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError,
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{address::Address, ergo_box::box_value::BoxValue, token::TokenAmount},
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
    let is_reward_token_rotation = reward_token_id != &in_oracle_box.reward_token().token_id;
    let (wallet_target_balance, wallet_target_tokens) = if is_reward_token_rotation {
        (
            tx_fee.checked_add(&BoxValue::SAFE_USER_MIN).unwrap(),
            vec![outbox_reward_tokens.clone().into()],
        )
    } else {
        (tx_fee, vec![])
    };
    let mut unspent_boxes = node_api.get_unspent_boxes_by_address(
        &oracle_address.to_base58(),
        wallet_target_balance,
        wallet_target_tokens,
    )?;
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

    #[test]
    fn test_subsequent_publish_datapoint_rotation_requests_new_reward_token() {
        use std::cell::RefCell;

        use crate::node_interface::node_api::NodeApiError;
        use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
        use ergo_lib::chain::transaction::Transaction;
        use ergo_lib::ergo_chain_types::Digest32;
        use ergo_lib::wallet::Wallet;
        use ergo_node_interface::P2PKAddressString;

        struct SelectingMockNodeApi<'a> {
            inner: MockNodeApi<'a>,
            requested_target_tokens: RefCell<Vec<Vec<Token>>>,
        }

        impl NodeApiTrait for SelectingMockNodeApi<'_> {
            fn get_unspent_boxes_by_address_with_token_filter_option(
                &self,
                address: &P2PKAddressString,
                target_balance: BoxValue,
                target_tokens: Vec<Token>,
                filter_boxes_token_ids: Vec<TokenId>,
            ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
                self.inner
                    .get_unspent_boxes_by_address_with_token_filter_option(
                        address,
                        target_balance,
                        target_tokens,
                        filter_boxes_token_ids,
                    )
            }

            fn get_unspent_boxes_by_address(
                &self,
                _address: &P2PKAddressString,
                target_balance: BoxValue,
                target_tokens: Vec<Token>,
            ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
                self.requested_target_tokens
                    .borrow_mut()
                    .push(target_tokens.clone());
                let selection = SimpleBoxSelector::new().select(
                    self.inner.unspent_boxes.clone(),
                    target_balance,
                    target_tokens.as_slice(),
                )?;
                Ok(selection.boxes.to_vec())
            }

            fn get_unspent_boxes_by_token_id(
                &self,
                token_id: &TokenId,
            ) -> Result<Vec<ErgoBox>, NodeApiError> {
                self.inner.get_unspent_boxes_by_token_id(token_id)
            }

            fn get_state_context(&self) -> Result<ErgoStateContext, NodeApiError> {
                self.inner.get_state_context()
            }

            fn get_wallet(&self) -> Result<Wallet, NodeApiError> {
                self.inner.get_wallet()
            }

            fn sign_transaction(
                &self,
                transaction_context: TransactionContext<UnsignedTransaction>,
            ) -> Result<Transaction, NodeApiError> {
                self.inner.sign_transaction(transaction_context)
            }

            fn submit_transaction(&self, tx: &Transaction) -> Result<TxId, NodeApiError> {
                self.inner.submit_transaction(tx)
            }

            fn sign_and_submit_transaction(
                &self,
                transaction_context: TransactionContext<UnsignedTransaction>,
            ) -> Result<TxId, NodeApiError> {
                self.inner.sign_and_submit_transaction(transaction_context)
            }
        }

        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let new_reward_token_id =
            RewardTokenId::from_token_id_unchecked(TokenId::from(Digest32::from([0xCC; 32])));
        assert_ne!(new_reward_token_id, token_ids.reward_token_id);
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

        // First box has enough ERG for tx_fee but no reward token; second box holds the rotated reward token.
        let erg_only_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10000).unwrap(),
            None,
        );
        let reward_token_box = make_wallet_unspent_box(
            secret.public_image(),
            *BASE_FEE,
            Some(
                vec![Token {
                    token_id: new_reward_token_id.token_id(),
                    amount: 1u64.try_into().unwrap(),
                }]
                .try_into()
                .unwrap(),
            ),
        );
        let submit_tx_mock = SubmitTxMock::default();
        let selecting_node_api = SelectingMockNodeApi {
            inner: MockNodeApi {
                unspent_boxes: vec![erg_only_box, reward_token_box],
                ctx: ctx.clone(),
                secrets: vec![secret.clone().into()],
                submitted_txs: &submit_tx_mock.transactions,
                chain_submit_tx: None,
            },
            requested_target_tokens: RefCell::new(vec![]),
        };

        let datapoint_source = MockDatapointSource {
            datapoint: 201.into(),
        };
        let (action, _) = build_subsequent_publish_datapoint_action(
            &oracle_box,
            &selecting_node_api,
            height,
            oracle_address.clone(),
            change_address.address(),
            &datapoint_source,
            pool_box_epoch_id,
            &new_reward_token_id,
        )
        .unwrap();

        assert_eq!(
            selecting_node_api.requested_target_tokens.borrow()[0],
            vec![Token {
                token_id: new_reward_token_id.token_id(),
                amount: 1u64.try_into().unwrap(),
            }]
        );

        let _signed_tx = selecting_node_api
            .sign_transaction(action.transaction_context)
            .unwrap();

        // When reward token has not rotated, no target tokens are requested from the wallet.
        let (same_token_action, _) = build_subsequent_publish_datapoint_action(
            &oracle_box,
            &selecting_node_api,
            height,
            oracle_address,
            change_address.address(),
            &datapoint_source,
            pool_box_epoch_id,
            &token_ids.reward_token_id,
        )
        .unwrap();
        assert!(selecting_node_api.requested_target_tokens.borrow()[1].is_empty());
        let _signed_same_tx = selecting_node_api
            .sign_transaction(same_token_action.transaction_context)
            .unwrap();
    }
}
