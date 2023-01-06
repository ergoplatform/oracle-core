use crate::oracle_state::LiveEpochState;
use crate::oracle_state::LocalDatapointState::Collected;
use crate::oracle_state::LocalDatapointState::Posted;
use crate::oracle_types::BlockHeight;
use crate::oracle_types::EpochLength;
use crate::pool_commands::PoolCommand;

pub struct EpochState {
    epoch_start_height: u64,
}

/// Enum for the state that the oracle pool is currently in
#[derive(Debug, Clone)]
pub enum PoolState {
    NeedsBootstrap,
    LiveEpoch(LiveEpochState),
}

pub fn process(
    pool_state: PoolState,
    epoch_length: EpochLength,
    current_height: BlockHeight,
) -> Option<PoolCommand> {
    let min_start_height = current_height - epoch_length;
    match pool_state {
        PoolState::NeedsBootstrap => {
            log::warn!(
                "No oracle pool found, needs bootstrap or wait for bootstrap txs to be on-chain"
            );
            None
        }
        PoolState::LiveEpoch(live_epoch) => {
            log::debug!("Height {current_height}. Live epoch state: {live_epoch:?}");
            if let Some(local_datapoint_box_state) = live_epoch.local_datapoint_box_state {
                match local_datapoint_box_state {
                    Collected { height: _ } => {
                        // publish datapoint after some blocks have passed after the pool box published
                        // to avoid some oracle box become stale on the next refresh
                        // (datapoint posted on the first block of the epoch go out of the epoch window too fast)
                        if current_height.0
                            > live_epoch.latest_pool_box_height.0 + (epoch_length.0 as u32) / 2
                        {
                            Some(PoolCommand::PublishSubsequentDataPoint { republish: false })
                        } else {
                            None
                        }
                    }
                    Posted { epoch_id, height } => {
                        if height < min_start_height || epoch_id != live_epoch.pool_box_epoch_id {
                            Some(PoolCommand::PublishSubsequentDataPoint { republish: true })
                        } else if live_epoch.latest_pool_box_height < min_start_height
                            && epoch_id == live_epoch.pool_box_epoch_id
                        {
                            Some(PoolCommand::Refresh)
                        } else {
                            None
                        }
                    }
                }
            } else {
                // no local datapoint found
                Some(PoolCommand::PublishFirstDataPoint)
            }
        }
    }
}

// TODO: add tests
