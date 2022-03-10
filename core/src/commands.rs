use crate::actions::PoolAction;
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
) -> Result<PoolAction, PoolCommandError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use ergo_lib::chain::ergo_box::ErgoBox;

    use crate::Result;

    use super::*;

    struct LiveEpochStageMock {
        refresh_box: ErgoBox,
        pool_box: ErgoBox,
    }

    impl LiveEpochStage for LiveEpochStageMock {
        fn get_refresh_box(&self) -> Result<ErgoBox> {
            Ok(self.refresh_box)
        }

        fn get_pool_box(&self) -> Result<ErgoBox> {
            Ok(self.pool_box)
        }
    }

    struct DatapointStageMock {
        datapoints: Vec<ErgoBox>,
    }

    impl DatapointStage for DatapointStageMock {
        fn get_oracle_datapoint_boxes(&self) -> Result<Vec<ErgoBox>> {
            Ok(self.datapoints)
        }
    }

    fn make_refresh_box() -> ErgoBox {
        todo!()
    }

    fn make_pool_box() -> ErgoBox {
        todo!()
    }

    fn make_datapoint_box() -> ErgoBox {
        todo!()
    }

    #[test]
    fn test_refresh_pool() {
        let refresh_box = make_refresh_box();
        let pool_box = make_pool_box();
        let datapoints = vec![make_datapoint_box()];
        let live_epoch_stage_mock = LiveEpochStageMock {
            refresh_box,
            pool_box,
        };
        let datapoint_stage_mock = DatapointStageMock { datapoints };
        let action = build_refresh_action(live_epoch_stage_mock, datapoint_stage_mock).unwrap();
        // TODO: check action data is as expected
    }
}
