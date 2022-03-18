use std::convert::TryInto;

use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::wallet::box_selector::BoxSelection;
use ergo_lib::wallet::tx_builder::TxBuilder;

use crate::actions::PoolAction;
use crate::actions::RefreshAction;
use crate::oracle_state::DatapointStage;
use crate::oracle_state::LiveEpochStage;
use crate::oracle_state::StageError;
use crate::BlockHeight;

pub enum PoolCommand {
    Bootstrap,
    Refresh,
}

#[derive(Debug, From)]
pub enum PoolCommandError {
    StageError(StageError),
}

pub fn build_action<A: LiveEpochStage, B: DatapointStage>(
    cmd: PoolCommand,
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
    height: BlockHeight,
) -> Result<PoolAction, PoolCommandError> {
    match cmd {
        PoolCommand::Bootstrap => todo!(),
        PoolCommand::Refresh => {
            build_refresh_action(live_epoch_stage_src, datapoint_stage_src, height).map(Into::into)
        }
    }
}

pub fn build_refresh_action<A: LiveEpochStage, B: DatapointStage>(
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
    height: BlockHeight,
) -> Result<RefreshAction, PoolCommandError> {
    let in_pool_box = live_epoch_stage_src.get_pool_box()?;
    let in_refresh_box = live_epoch_stage_src.get_refresh_box()?;
    let in_oracle_boxes = datapoint_stage_src.get_oracle_datapoint_boxes()?;
    let out_pool_box = build_out_pool_box()?;
    let out_refresh_box = build_out_pool_box()?;
    let out_oracle_boxes = build_out_oracle_boxes()?;

    // TODO: get all unspent boxes via NodeInterface::unspent_boxes()
    // TODO: use BoxSelector to select input boxes for tx fee
    // TODO: append selected input boxes to the box_selection below

    let mut input_boxes = vec![in_pool_box, in_refresh_box];
    input_boxes.append(&mut in_oracle_boxes);
    let box_selection = BoxSelection {
        boxes: input_boxes.try_into().unwrap(),
        change_boxes: vec![],
    };

    let mut output_candidates = vec![out_pool_box, out_refresh_box];
    output_candidates.append(&mut out_oracle_boxes);

    let tx_fee = BoxValue::SAFE_USER_MIN;

    // TODO: HOW TO DETERMINE CHANGE ADDRESS??? via /wallet/status (changeAddress field) ?

    let b = TxBuilder::new(
        box_selection,
        output_candidates,
        height as u32,
        tx_fee,
        change_address,
        BoxValue::MIN,
    );
    let tx = b.build()?;
    Ok(RefreshAction { tx })
}

fn build_out_pool_box() -> Result<ErgoBoxCandidate, PoolCommandError> {
    todo!()
}

fn build_out_refresh_box() -> Result<ErgoBoxCandidate, PoolCommandError> {
    todo!()
}

fn build_out_oracle_boxes() -> Result<Vec<ErgoBoxCandidate>, PoolCommandError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::oracle_state::Result;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenAmount;
    use ergo_lib::ergotree_ir::chain::token::TokenId;

    use crate::BlockHeight;

    use super::*;

    struct LiveEpochStageMock {
        refresh_box: ErgoBox,
        pool_box: ErgoBox,
    }

    impl LiveEpochStage for LiveEpochStageMock {
        fn get_refresh_box(&self) -> Result<ErgoBox> {
            Ok(self.refresh_box.clone())
        }

        fn get_pool_box(&self) -> Result<ErgoBox> {
            Ok(self.pool_box.clone())
        }
    }

    struct DatapointStageMock {
        datapoints: Vec<ErgoBox>,
    }

    impl DatapointStage for DatapointStageMock {
        fn get_oracle_datapoint_boxes(&self) -> Result<Vec<ErgoBox>> {
            Ok(self.datapoints.clone())
        }
    }

    fn make_refresh_box(refresh_nft: &TokenId, reward_token: Token, value: BoxValue) -> ErgoBox {
        ErgoBox::new(
            value,
            ergo_tree,
            tokens,
            additional_registers,
            creation_height,
            transaction_id,
            index,
        )
        .unwrap()
    }

    fn make_pool_box(
        epoch_start_height: BlockHeight,
        datapoint: u64,
        epoch_counter: u32,
        refresh_nft: TokenId,
        value: BoxValue,
    ) -> ErgoBox {
        todo!()
    }

    fn make_datapoint_box() -> ErgoBox {
        todo!()
    }

    #[test]
    fn test_refresh_pool() {
        let reward_token_id =
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap();
        let reward_token_amt: TokenAmount = 100u64.try_into().unwrap();
        let reward_token: Token = (reward_token_id, reward_token_amt).into();
        let refresh_nft =
            TokenId::from_base64("VGpXblpyNHU3eCFBJUQqRy1LYU5kUmdVa1hwMnM1djg=").unwrap();
        let refresh_box = make_refresh_box(&refresh_nft, reward_token, BoxValue::SAFE_USER_MIN);
        let pool_box = make_pool_box(1, 1, 1, refresh_nft, BoxValue::SAFE_USER_MIN);
        let datapoints = vec![make_datapoint_box()];
        let live_epoch_stage_mock = LiveEpochStageMock {
            refresh_box,
            pool_box: pool_box.clone(),
        };
        let datapoint_stage_mock = DatapointStageMock { datapoints };
        let action = build_refresh_action(live_epoch_stage_mock, datapoint_stage_mock).unwrap();
        assert_eq!(action.in_pool_box, pool_box);
    }
}
