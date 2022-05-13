use std::convert::TryInto;

use derive_more::From;
use ergo_lib::{
    chain::ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
    ergotree_interpreter::sigma_protocol::prover::ContextExtension,
    ergotree_ir::chain::{
        address::Address,
        ergo_box::{
            box_value::BoxValue,
            NonMandatoryRegisterId::{R4, R5, R6},
        },
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
    box_kind::{OracleBox, PoolBox},
    oracle_state::{LocalDatapointBoxSource, PoolBoxSource, StageError},
    wallet::WalletDataSource,
};

#[derive(Debug, Error, From)]
pub enum PublishDatapointActionError {
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("Oracle box has no reward token")]
    NoRewardToken,
    #[error("tx builder error: {0}")]
    TxBuilder(TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("node error: {0}")]
    Node(NodeError),
    #[error("box selector error: {0}")]
    BoxSelector(BoxSelectorError),
}

pub fn build_publish_datapoint_action(
    pool_box_source: &dyn PoolBoxSource,
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    wallet: &dyn WalletDataSource,
    height: u32,
    change_address: Address,
    new_datapoint: i64,
) -> Result<PublishDataPointAction, PublishDatapointActionError> {
    let in_pool_box = pool_box_source.get_pool_box()?;
    let in_oracle_box = local_datapoint_box_source.get_local_oracle_datapoint_box()?;
    if *in_oracle_box.reward_token().amount.as_u64() == 0 {
        return Err(PublishDatapointActionError::NoRewardToken);
    }

    // Build the single output box
    let mut builder = ErgoBoxCandidateBuilder::new(
        in_oracle_box.get_box().value,
        in_oracle_box.get_box().ergo_tree.clone(),
        height,
    );
    let new_epoch_counter: i32 = (in_pool_box.epoch_counter() + 1) as i32;
    builder.set_register_value(R4, in_oracle_box.public_key().into());
    builder.set_register_value(R5, new_epoch_counter.into());
    builder.set_register_value(
        R6,
        compute_new_datapoint(new_datapoint, in_oracle_box.rate() as i64).into(),
    );
    builder.add_token(in_oracle_box.oracle_token().clone());
    builder.add_token(in_oracle_box.reward_token().clone());
    let output_candidate = builder.build()?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let tx_fee = BoxValue::SAFE_USER_MIN;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;
    let mut input_boxes = vec![in_oracle_box.get_box()];
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
    use std::convert::TryFrom;
    use std::convert::TryInto;

    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::chain::transaction::TxIoVec;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenId;
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::mir::expr::Expr;
    use ergo_lib::ergotree_ir::sigma_protocol::dlog_group::EcPoint;
    use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use ergo_node_interface::node_interface::NodeError;
    use sigma_test_util::force_any_val;

    use crate::box_kind::OracleBoxWrapper;
    use crate::box_kind::PoolBoxWrapper;
    use crate::contracts::oracle::OracleContract;
    use crate::contracts::pool::PoolContract;
    use crate::contracts::refresh::RefreshContract;
    use crate::oracle_state::StageError;

    use super::*;

    #[derive(Clone)]
    struct PoolBoxMock {
        pool_box: PoolBoxWrapper,
    }

    impl PoolBoxSource for PoolBoxMock {
        fn get_pool_box(&self) -> std::result::Result<PoolBoxWrapper, StageError> {
            Ok(self.pool_box.clone())
        }
    }

    #[derive(Clone)]
    struct OracleBoxMock {
        oracle_box: OracleBoxWrapper,
    }

    impl LocalDatapointBoxSource for OracleBoxMock {
        fn get_local_oracle_datapoint_box(
            &self,
        ) -> std::result::Result<OracleBoxWrapper, StageError> {
            Ok(self.oracle_box.clone())
        }
    }

    #[derive(Clone)]
    struct WalletDataMock {
        unspent_boxes: Vec<ErgoBox>,
    }

    impl WalletDataSource for WalletDataMock {
        fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, NodeError> {
            Ok(self.unspent_boxes.clone())
        }
    }

    fn make_pool_box(
        datapoint: i64,
        epoch_counter: i32,
        pool_nft_token_id: TokenId,
        value: BoxValue,
        creation_height: u32,
    ) -> PoolBoxWrapper {
        let tokens = [Token::from((
            pool_nft_token_id.clone(),
            1u64.try_into().unwrap(),
        ))]
        .into();
        ErgoBox::new(
            value,
            PoolContract::new().ergo_tree(),
            Some(tokens),
            NonMandatoryRegisters::new(
                vec![
                    (NonMandatoryRegisterId::R4, Constant::from(datapoint)),
                    (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
                ]
                .into_iter()
                .collect(),
            )
            .unwrap(),
            creation_height,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
        .try_into()
        .unwrap()
    }

    fn make_datapoint_box(
        pub_key: EcPoint,
        datapoint: i64,
        epoch_counter: i32,
        oracle_token_id: TokenId,
        reward_token: Token,
        value: BoxValue,
        creation_height: u32,
    ) -> ErgoBox {
        let tokens = vec![
            Token::from((oracle_token_id.clone(), 1u64.try_into().unwrap())),
            reward_token,
        ]
        .try_into()
        .unwrap();
        ErgoBox::new(
            value,
            OracleContract::new().ergo_tree(),
            Some(tokens),
            NonMandatoryRegisters::new(
                vec![
                    (NonMandatoryRegisterId::R4, Constant::from(pub_key)),
                    (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
                    (NonMandatoryRegisterId::R6, Constant::from(datapoint)),
                ]
                .into_iter()
                .collect(),
            )
            .unwrap(),
            creation_height,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    fn make_wallet_unspent_box(pub_key: ProveDlog, value: BoxValue) -> ErgoBox {
        let c: Constant = pub_key.into();
        let expr: Expr = c.into();
        ErgoBox::new(
            value,
            ErgoTree::try_from(expr).unwrap(),
            None,
            NonMandatoryRegisters::empty(),
            1,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    fn find_input_boxes(
        tx: UnsignedTransaction,
        available_boxes: Vec<ErgoBox>,
    ) -> TxIoVec<ErgoBox> {
        tx.inputs.mapped(|i| {
            available_boxes
                .clone()
                .into_iter()
                .find(|b| b.box_id() == i.box_id)
                .unwrap()
        })
    }

    #[test]
    fn test_publish_datapoint() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let refresh_contract = RefreshContract::new();
        let reward_token_id =
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap();
        let pool_nft_token_id = refresh_contract.pool_nft_token_id();
        dbg!(&reward_token_id);
        let in_pool_box = make_pool_box(
            200,
            1,
            pool_nft_token_id,
            BoxValue::SAFE_USER_MIN,
            height - 32, // from previous epoch
        );
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let oracle_pub_key = secret.public_image().h;

        let pool_box_mock = PoolBoxMock {
            pool_box: in_pool_box,
        };

        let oracle_box = make_datapoint_box(
            *oracle_pub_key,
            200,
            1,
            refresh_contract.oracle_nft_token_id(),
            Token::from((reward_token_id, 5u64.try_into().unwrap())),
            BoxValue::SAFE_USER_MIN.checked_mul_u32(100).unwrap(),
            height - 9,
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
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };
        let action = build_publish_datapoint_action(
            &pool_box_mock,
            &local_datapoint_box_source,
            &wallet_mock,
            height,
            change_address,
            210,
        )
        .unwrap();

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box(),
            local_datapoint_box_source
                .get_local_oracle_datapoint_box()
                .unwrap()
                .get_box(),
        ];
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes),
            None,
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
