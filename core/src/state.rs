#![allow(unused_imports)]

use crate::actions::CollectionError;
use crate::datapoint_source::DataPointSource;
use crate::oracle_state::DatapointState;
use crate::oracle_state::LiveEpochState;
use crate::oracle_state::OraclePool;
use crate::oracle_state::PreparationState;
use crate::oracle_state::Stage;
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

pub fn process(pool_state: PoolState, height: u64) -> Result<Option<PoolCommand>, StageError> {
    match pool_state {
        PoolState::NeedsBootstrap => {
            log::warn!(
                "No oracle pool found, needs bootstrap or wait for bootstrap txs to be on-chain"
            );
            Ok(None)
        }
        PoolState::LiveEpoch(live_epoch) => {
            let epoch_is_over =
                height >= live_epoch.epoch_ends && live_epoch.commit_datapoint_in_epoch;
            if epoch_is_over {
                log::info!("Epoch is over, calling refresh");
                Ok(Some(PoolCommand::Refresh))
            } else if !live_epoch.commit_datapoint_in_epoch {
                log::info!("Commiting datapoint...");
                Ok(Some(PoolCommand::PublishDataPoint))
            } else {
                Ok(None)
            }
        }
    }
}
