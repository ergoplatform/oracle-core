use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_state::LiveEpochState;
use crate::oracle_state::StageError;
use crate::pool_commands::PoolCommand;
use anyhow::Result;

pub struct EpochState {
    epoch_start_height: u64,
}

/// Enum for the state that the oracle pool is currently in
#[derive(Debug, Clone)]
pub enum PoolState {
    NeedsBootstrap,
    LiveEpoch(LiveEpochState),
}

pub fn process(pool_state: PoolState, height: u32) -> Result<Option<PoolCommand>, StageError> {
    match pool_state {
        PoolState::NeedsBootstrap => {
            log::warn!(
                "No oracle pool found, needs bootstrap or wait for bootstrap txs to be on-chain"
            );
            Ok(None)
        }
        PoolState::LiveEpoch(live_epoch) => {
            let epoch_length = ORACLE_CONFIG
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .epoch_length() as u32;
            let epoch_is_over = height >= live_epoch.latest_pool_box_height + epoch_length
                && live_epoch.commit_datapoint_in_epoch;
            if epoch_is_over {
                log::info!(
                    "Height {height}. Epoch id {}, previous epoch ended (pool box) at {} + epoch lengh {epoch_length}, calling refresh action",
                    live_epoch.epoch_id,
                    live_epoch.latest_pool_box_height,
                );
                Ok(Some(PoolCommand::Refresh))
            } else if !live_epoch.commit_datapoint_in_epoch {
                log::info!("Height {height}. Publishing datapoint...");
                Ok(Some(PoolCommand::PublishDataPoint))
            } else {
                Ok(None)
            }
        }
    }
}
