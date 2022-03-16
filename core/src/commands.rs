use crate::actions::PoolAction;
use crate::actions::RefreshAction;
use crate::oracle_state::DatapointStage;
use crate::oracle_state::LiveEpochStage;

pub enum PoolCommand {
    Bootstrap,
    Refresh,
}

#[derive(Debug, Clone)]
pub enum PoolCommandError {}

pub fn build_action<A: LiveEpochStage, B: DatapointStage>(
    cmd: PoolCommand,
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
) -> Result<PoolAction, PoolCommandError> {
    todo!()
}

pub fn build_refresh_action<A: LiveEpochStage, B: DatapointStage>(
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
) -> Result<RefreshAction, PoolCommandError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenAmount;
    use ergo_lib::ergotree_ir::chain::token::TokenId;

    use crate::BlockHeight;
    use crate::Result;

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
        todo!()
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

    #[ignore = "make it green"]
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
        assert_eq!(action.pool_box, pool_box);
    }
}
