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
    builder.set_register_value(R6, new_datapoint.into());
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
