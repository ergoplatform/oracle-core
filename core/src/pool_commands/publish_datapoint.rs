use std::convert::TryFrom;

use ergo_lib::{
    chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError,
    ergo_chain_types::EcPoint,
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{address::Address, token::TokenAmount},
    wallet::{
        box_selector::{BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use thiserror::Error;

use crate::{
    actions::PublishDataPointAction,
    box_kind::{make_oracle_box_candidate, OracleBox, OracleBoxWrapper, OracleBoxWrapperInputs},
    contracts::oracle::{OracleContract, OracleContractError},
    datapoint_source::{DataPointSource, DataPointSourceError},
    oracle_config::BASE_FEE,
    oracle_state::DataSourceError,
    oracle_types::{BlockHeight, EpochCounter},
    spec_token::{OracleTokenId, RewardTokenId, SpecToken},
    wallet::{WalletDataError, WalletDataSource},
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
    #[error("WalletData error: {0}")]
    WalletData(#[from] WalletDataError),
    #[error("box selector error: {0}")]
    BoxSelector(#[from] BoxSelectorError),
    #[error("datapoint source error: {0}")]
    DataPointSource(#[from] DataPointSourceError),
    #[error("oracle contract error: {0}")]
    OracleContract(#[from] OracleContractError),
}

pub fn build_subsequent_publish_datapoint_action(
    local_datapoint_box: &OracleBoxWrapper,
    wallet: &dyn WalletDataSource,
    height: BlockHeight,
    change_address: Address,
    datapoint_source: &dyn DataPointSource,
    new_epoch_counter: EpochCounter,
    reward_token_id: &RewardTokenId,
) -> Result<PublishDataPointAction, PublishDatapointActionError> {
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

    let mut unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let tx_fee = *BASE_FEE;
    let box_selector = SimpleBoxSelector::new();
    let target_tokens = vec![
        in_oracle_box.oracle_token().into(),
        outbox_reward_tokens.into(),
    ];
    let target_balace = in_oracle_box.get_box().value.checked_add(&tx_fee).unwrap();
    unspent_boxes.push(in_oracle_box.get_box().clone());
    let selection = box_selector.select(unspent_boxes, target_balace, target_tokens.as_slice())?;
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
    Ok(PublishDataPointAction { tx })
}

#[allow(clippy::too_many_arguments)]
pub fn build_publish_first_datapoint_action(
    wallet: &dyn WalletDataSource,
    height: BlockHeight,
    change_address: Address,
    public_key: EcPoint,
    inputs: OracleBoxWrapperInputs,
    datapoint_source: &dyn DataPointSource,
) -> Result<PublishDataPointAction, PublishDatapointActionError> {
    let new_datapoint = datapoint_source.get_datapoint()?;
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
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

    let wallet_boxes_selection = box_selector.select(
        unspent_boxes.clone(),
        target_balance,
        &[oracle_token.clone().into(), reward_token.clone().into()],
    )?;

    let output_candidate = make_oracle_box_candidate(
        &contract,
        public_key,
        new_datapoint,
        EpochCounter(1),
        oracle_token,
        reward_token,
        min_storage_rent,
        height,
    )?;

    let box_id = wallet_boxes_selection.boxes.first().box_id();
    let mut tx_builder = TxBuilder::new(
        wallet_boxes_selection,
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
    Ok(PublishDataPointAction { tx })
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use crate::box_kind::PoolBox;
    use crate::contracts::oracle::OracleContractParameters;
    use crate::contracts::pool::PoolContractParameters;
    use crate::oracle_state::PoolBoxSource;
    use crate::oracle_types::EpochLength;
    use crate::pool_commands::test_utils::{
        find_input_boxes, generate_token_ids, make_datapoint_box, make_pool_box,
        make_wallet_unspent_box, PoolBoxMock, WalletDataMock,
    };
    use crate::spec_token::TokenIdKind;
    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::{BoxTokens, ErgoBox, NonMandatoryRegisters};
    use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::mir::expr::Expr;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use sigma_test_util::force_any_val;

    #[derive(Debug)]
    struct MockDatapointSource {
        datapoint: i64,
    }

    impl DataPointSource for MockDatapointSource {
        fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
            Ok(self.datapoint)
        }
    }

    #[test]
    fn test_subsequent_publish_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let oracle_contract_parameters = OracleContractParameters::default();
        let pool_contract_parameters = PoolContractParameters::default();
        let pool_box_epoch_id = EpochCounter(1);
        let in_pool_box = make_pool_box(
            200,
            pool_box_epoch_id,
            *BASE_FEE,
            height - EpochLength(32), // from previous epoch
            &pool_contract_parameters,
            &token_ids,
        );
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let oracle_pub_key = secret.public_image().h;

        let pool_box_mock = PoolBoxMock {
            pool_box: in_pool_box,
        };

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
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
            change_address: change_address.clone(),
        };

        let datapoint_source = MockDatapointSource { datapoint: 201 };
        let action = build_subsequent_publish_datapoint_action(
            &oracle_box,
            &wallet_mock,
            height,
            change_address.address(),
            &datapoint_source,
            pool_box_epoch_id,
            &token_ids.reward_token_id,
        )
        .unwrap();

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box().clone(),
            oracle_box.get_box().clone(),
        ];
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes.clone()),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
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
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
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
        let action = build_publish_first_datapoint_action(
            &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
                change_address: change_address.clone(),
            },
            height,
            change_address.address(),
            *secret.public_image().h,
            oracle_box_wrapper_inputs,
            &MockDatapointSource { datapoint: 201 },
        )
        .unwrap();

        assert_eq!(
            action.tx.output_candidates.first().value,
            oracle_contract_parameters.min_storage_rent
        );

        let tx_context =
            TransactionContext::new(action.tx.clone(), unspent_boxes, Vec::new()).unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }

    #[test]
    fn test_subsequent_publish_datapoint_with_minted_reward_token() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let token_ids = generate_token_ids();
        let minted_reward_token_id =
            RewardTokenId::from_token_id_unchecked(force_any_val::<TokenId>());
        let oracle_contract_parameters = OracleContractParameters::default();
        let pool_contract_parameters = PoolContractParameters::default();
        let pool_box_epoch_id = EpochCounter(1);
        dbg!(&token_ids);
        dbg!(&minted_reward_token_id);
        let in_pool_box = make_pool_box(
            200,
            pool_box_epoch_id,
            *BASE_FEE,
            height - EpochLength(32), // from previous epoch
            &pool_contract_parameters,
            &token_ids,
        );
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let oracle_pub_key = secret.public_image().h;

        let pool_box_mock = PoolBoxMock {
            pool_box: in_pool_box,
        };

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
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
            change_address: change_address.clone(),
        };

        let datapoint_source = MockDatapointSource { datapoint: 201 };
        let action = build_subsequent_publish_datapoint_action(
            &oracle_box,
            &wallet_mock,
            height,
            change_address.address(),
            &datapoint_source,
            pool_box_epoch_id,
            &minted_reward_token_id,
        )
        .unwrap();

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box().clone(),
            oracle_box.get_box().clone(),
        ];
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes.clone()),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
