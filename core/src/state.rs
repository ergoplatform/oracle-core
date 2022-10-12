use crate::oracle_state::LiveEpochState;
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

pub fn process(pool_state: PoolState, epoch_length: u32, height: u32) -> Option<PoolCommand> {
    match pool_state {
        PoolState::NeedsBootstrap => {
            log::warn!(
                "No oracle pool found, needs bootstrap or wait for bootstrap txs to be on-chain"
            );
            None
        }
        PoolState::LiveEpoch(live_epoch) => {
            if let Some(local_datapoint_box_state) = live_epoch.local_datapoint_box_state {
                if local_datapoint_box_state.epoch_id != live_epoch.epoch_id {
                    log::info!("Height {height}. Publishing datapoint. Last datapoint was published at {}, current epoch id is {})...", local_datapoint_box_state.epoch_id, live_epoch.epoch_id);
                    Some(PoolCommand::PublishSubsequentDataPoint { republish: false })
                } else if local_datapoint_box_state.height < height - epoch_length {
                    log::info!(
                        "Height {height}. Re-publishing datapoint (last one is too old, at {})...",
                        local_datapoint_box_state.height
                    );
                    Some(PoolCommand::PublishSubsequentDataPoint { republish: true })
                } else if height >= live_epoch.latest_pool_box_height + epoch_length {
                    log::info!("Height {height}. Refresh action. Height {height}. Last epoch id {}, previous epoch started (pool box) at {}", live_epoch.epoch_id, live_epoch.latest_pool_box_height,);
                    Some(PoolCommand::Refresh)
                } else {
                    None
                }
            } else {
                // no last local datapoint posted
                log::info!("Height {height}. Publishing datapoint (first)...");
                Some(PoolCommand::PublishFirstDataPoint)
            }
        }
    }
}

// TODO: add tests
