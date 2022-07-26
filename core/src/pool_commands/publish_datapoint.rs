use std::convert::{TryFrom, TryInto};

use derive_more::From;
use ergo_lib::{
    chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError,
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::{
        chain::{
            address::Address,
            ergo_box::box_value::BoxValue,
            token::{Token, TokenAmount},
        },
        sigma_protocol::sigma_boolean::ProveDlog,
    },
    wallet::{
        box_selector::{BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::{
    actions::PublishDataPointAction,
    box_kind::{make_oracle_box_candidate, OracleBox, OracleBoxWrapperInputs, PoolBox},
    contracts::oracle::{OracleContract, OracleContractError},
    datapoint_source::{DataPointSource, DataPointSourceError},
    oracle_state::{LocalDatapointBoxSource, PoolBoxSource, StageError},
    wallet::WalletDataSource,
};

use super::PublishDataPointCommandInputs;

#[derive(Debug, Error, From)]
pub enum PublishDatapointActionError {
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("Oracle box has no reward token")]
    NoRewardTokenInOracleBox,
    #[error("tx builder error: {0}")]
    TxBuilder(TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("node error: {0}")]
    Node(NodeError),
    #[error("box selector error: {0}")]
    BoxSelector(BoxSelectorError),
    #[error("datapoint source error: {0}")]
    DataPointSource(DataPointSourceError),
    #[error("oracle contract error: {0}")]
    OracleContract(OracleContractError),
}

pub fn build_publish_datapoint_action(
    pool_box_source: &dyn PoolBoxSource,
    inputs: PublishDataPointCommandInputs,
    wallet: &dyn WalletDataSource,
    datapoint_source: &dyn DataPointSource,
    height: u32,
    change_address: Address,
) -> Result<PublishDataPointAction, PublishDatapointActionError> {
    let new_datapoint = datapoint_source.get_datapoint()?;
    let epoch_counter = pool_box_source.get_pool_box()?.epoch_counter();
    match inputs {
        PublishDataPointCommandInputs::LocalDataPointBoxExists(local_datapoint_box_source) => {
            build_subsequent_publish_datapoint_action(
                local_datapoint_box_source,
                wallet,
                epoch_counter,
                height,
                change_address,
                new_datapoint,
            )
        }
        PublishDataPointCommandInputs::FirstDataPoint {
            public_key,
            oracle_box_wrapper_inputs: oracle_box_inputs,
        } => build_publish_first_datapoint_action(
            wallet,
            height,
            change_address,
            new_datapoint as u64,
            public_key,
            oracle_box_inputs,
        ),
    }
}

pub fn build_subsequent_publish_datapoint_action(
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    wallet: &dyn WalletDataSource,
    current_epoch_counter: u32,
    height: u32,
    change_address: Address,
    new_datapoint: i64,
) -> Result<PublishDataPointAction, PublishDatapointActionError> {
    let in_oracle_box = local_datapoint_box_source.get_local_oracle_datapoint_box()?;
    if *in_oracle_box.reward_token().amount.as_u64() == 0 {
        return Err(PublishDatapointActionError::NoRewardTokenInOracleBox);
    }
    let new_epoch_counter: u32 = current_epoch_counter + 1;

    let output_candidate = make_oracle_box_candidate(
        in_oracle_box.contract(),
        in_oracle_box.public_key(),
        compute_new_datapoint(new_datapoint, in_oracle_box.rate() as i64) as u64,
        new_epoch_counter,
        in_oracle_box.oracle_token(),
        in_oracle_box.reward_token(),
        in_oracle_box.get_box().value,
        height,
    )?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let tx_fee = BoxValue::SAFE_USER_MIN;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;
    let mut input_boxes = vec![in_oracle_box.get_box().clone()];
    input_boxes.append(selection.boxes.as_vec().clone().as_mut());
    let box_selection = BoxSelection {
        boxes: input_boxes.try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };
    let mut tx_builder = TxBuilder::new(
        box_selection,
        vec![output_candidate],
        height,
        tx_fee,
        change_address,
        BoxValue::MIN,
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
    height: u32,
    change_address: Address,
    new_datapoint: u64,
    public_key: ProveDlog,
    inputs: OracleBoxWrapperInputs,
) -> Result<PublishDataPointAction, PublishDatapointActionError> {
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let tx_fee = BoxValue::SAFE_USER_MIN;
    let box_selector = SimpleBoxSelector::new();
    let oracle_token = Token {
        token_id: inputs.oracle_token_id.clone(),
        amount: TokenAmount::try_from(1).unwrap(),
    };
    let reward_token = Token {
        token_id: inputs.reward_token_id.clone(),
        amount: TokenAmount::try_from(1).unwrap(),
    };

    // We need to deduct `2*tx_fee` from the wallet. `fee` goes to the output box and the remaining
    // for tx fees.
    let target_balance = tx_fee.checked_mul_u32(2).unwrap();

    let wallet_boxes_selection = box_selector.select(
        unspent_boxes.clone(),
        target_balance,
        &[oracle_token.clone(), reward_token.clone()],
    )?;

    let output_candidate = make_oracle_box_candidate(
        &OracleContract::new(inputs.into())?,
        public_key,
        new_datapoint,
        1,
        oracle_token,
        reward_token,
        BoxValue::SAFE_USER_MIN,
        height,
    )?;

    let box_id = wallet_boxes_selection.boxes.first().box_id();
    let mut tx_builder = TxBuilder::new(
        wallet_boxes_selection,
        vec![output_candidate],
        height,
        tx_fee,
        change_address,
        BoxValue::MIN,
    );

    // The following context value ensures that `outIndex` in the oracle contract is properly set.
    let ctx_ext = ContextExtension {
        values: vec![(0, 0i32.into())].into_iter().collect(),
    };
    tx_builder.set_context_extension(box_id, ctx_ext);
    let tx = tx_builder.build()?;
    Ok(PublishDataPointAction { tx })
}

fn compute_new_datapoint(datapoint: i64, old_datapoint: i64) -> i64 {
    // Difference calc
    let difference = datapoint as f64 / old_datapoint as f64;

    // If the new datapoint is twice as high, post the new datapoint
    #[allow(clippy::if_same_then_else)]
    if difference > 2.00 {
        datapoint
    }
    // If the new datapoint is half, post the new datapoint
    else if difference < 0.50 {
        datapoint
    }
    // TODO: remove 0.5% cap, kushti asked on TG:
    // >Lets run 2.0 with no delay in data update in the default data provider
    // >No, data provider currently cap oracle price change at 0.5 percent per epoch
    //
    // If the new datapoint is 0.49% to 50% lower, post 0.49% lower than old
    else if difference < 0.9951 {
        (old_datapoint as f64 * 0.9951) as i64
    }
    // If the new datapoint is 0.49% to 100% higher, post 0.49% higher than old
    else if difference > 1.0049 {
        (old_datapoint as f64 * 1.0049) as i64
    }
    // Else if the difference is within 0.49% either way, post the new datapoint
    else {
        datapoint
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use crate::contracts::oracle::OracleContractParameters;
    use crate::contracts::pool::PoolContractParameters;
    use crate::pool_commands::test_utils::{
        find_input_boxes, generate_token_ids, make_datapoint_box, make_pool_box,
        make_wallet_unspent_box, OracleBoxMock, PoolBoxMock, WalletDataMock,
    };
    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::{BoxTokens, ErgoBox, NonMandatoryRegisters};
    use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::mir::expr::Expr;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use sigma_test_util::force_any_val;

    #[derive(Debug)]
    struct MockDatapointSource {}

    impl DataPointSource for MockDatapointSource {
        fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
            Ok(201)
        }
    }

    #[test]
    fn test_subsequent_publish_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let token_ids = generate_token_ids();
        let reward_token_id = force_any_val::<TokenId>();
        let oracle_contract_parameters = OracleContractParameters::default();
        let pool_contract_parameters = PoolContractParameters::default();
        dbg!(&reward_token_id);
        let in_pool_box = make_pool_box(
            200,
            1,
            BoxValue::SAFE_USER_MIN,
            height - 32, // from previous epoch
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
            OracleBoxWrapperInputs::from((&oracle_contract_parameters, &token_ids));
        let oracle_box = (
            make_datapoint_box(
                *oracle_pub_key,
                200,
                1,
                &token_ids,
                BoxValue::SAFE_USER_MIN.checked_mul_u32(100).unwrap(),
                height - 9,
            ),
            oracle_box_wrapper_inputs,
        )
            .try_into()
            .unwrap();
        let local_datapoint_box_source = OracleBoxMock { oracle_box };

        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();

        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BoxValue::SAFE_USER_MIN.checked_mul_u32(10000).unwrap(),
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };

        let datapoint_source = MockDatapointSource {};
        let action = build_publish_datapoint_action(
            &pool_box_mock,
            PublishDataPointCommandInputs::LocalDataPointBoxExists(
                &local_datapoint_box_source as &dyn LocalDatapointBoxSource,
            ),
            &wallet_mock,
            &datapoint_source,
            height,
            change_address,
        )
        .unwrap();

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box().clone(),
            local_datapoint_box_source
                .get_local_oracle_datapoint_box()
                .unwrap()
                .get_box()
                .clone(),
        ];
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }

    #[test]
    fn test_first_publish_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;

        let token_ids = generate_token_ids();
        let tokens = BoxTokens::from_vec(vec![
            Token::from((
                token_ids.reward_token_id.clone(),
                100u64.try_into().unwrap(),
            )),
            Token::from((token_ids.oracle_token_id.clone(), 1u64.try_into().unwrap())),
        ])
        .unwrap();

        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let c: Constant = secret.public_image().into();
        let expr: Expr = c.into();
        let ergo_tree = ErgoTree::try_from(expr).unwrap();

        let value = BoxValue::SAFE_USER_MIN.checked_mul_u32(10000).unwrap();
        let box_with_tokens = ErgoBox::new(
            value,
            ergo_tree.clone(),
            Some(tokens),
            NonMandatoryRegisters::new(vec![].into_iter().collect()).unwrap(),
            height - 30,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap();
        let unspent_boxes = vec![
            box_with_tokens.clone(),
            ErgoBox::new(
                BoxValue::SAFE_USER_MIN,
                ergo_tree.clone(),
                None,
                NonMandatoryRegisters::new(vec![].into_iter().collect()).unwrap(),
                height - 9,
                force_any_val::<TxId>(),
                0,
            )
            .unwrap(),
        ];

        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();

        let oracle_contract_parameters = OracleContractParameters::default();
        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::from((&oracle_contract_parameters, &token_ids));
        let action = build_publish_first_datapoint_action(
            &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
            },
            height,
            change_address,
            100,
            secret.public_image(),
            oracle_box_wrapper_inputs,
        )
        .unwrap();

        let tx_context =
            TransactionContext::new(action.tx.clone(), unspent_boxes, Vec::new()).unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
