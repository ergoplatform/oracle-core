use crate::oracle_state::LiveEpochState;
use crate::oracle_state::LocalDatapointState::Collected;
use crate::oracle_state::LocalDatapointState::Posted;
use crate::pool_commands::PoolCommand;

pub struct EpochState {
    epoch_start_height: u64,
}

// TODO: remove NeedsBootstrap and use LiveEpochState?
/// Enum for the state that the oracle pool is currently in
#[derive(Debug, Clone)]
pub enum PoolState {
    NeedsBootstrap,
    LiveEpoch(LiveEpochState),
}

pub fn process(
    pool_state: PoolState,
    epoch_length: u32,
    current_height: u32,
) -> Option<PoolCommand> {
    match pool_state {
        PoolState::NeedsBootstrap => {
            log::warn!(
                "No oracle pool found, needs bootstrap or wait for bootstrap txs to be on-chain"
            );
            None
        }
        PoolState::LiveEpoch(live_epoch) => {
            if let Some(local_datapoint_box_state) = live_epoch.local_datapoint_box_state {
                match local_datapoint_box_state {
                    Collected { height: _ } => {
                        Some(PoolCommand::PublishSubsequentDataPoint { republish: false })
                    }
                    Posted {
                        epoch_id: _,
                        height,
                    } => {
                        if height < current_height - epoch_length {
                            Some(PoolCommand::PublishSubsequentDataPoint { republish: true })
                        } else if current_height >= live_epoch.latest_pool_box_height + epoch_length
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
