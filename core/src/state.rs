#![allow(unused_imports)]

use crate::actions::CollectionError;
use crate::commands::PoolCommand;
use crate::oracle_config::PoolParameters;
use crate::oracle_state::DatapointState;
use crate::oracle_state::LiveEpochState;
use crate::oracle_state::OraclePool;
use crate::oracle_state::PreparationState;
use crate::oracle_state::Stage;
use crate::oracle_state::StageError;
use crate::TokenID;
use anyhow::Result;

pub struct EpochState {
    epoch_start_height: u64,
}

pub enum PoolState {
    NeedsBootstrap,
    LiveEpoch(LiveEpochState),
}

pub fn process(
    pool_state: PoolState,
    // op: OraclePool,
    // parameters: PoolParameters,
    height: u64,
) -> Result<Option<PoolCommand>, StageError> {
    match pool_state {
        PoolState::NeedsBootstrap => todo!(),
        PoolState::LiveEpoch(live_epoch) => {
            let epoch_is_over =
                height >= live_epoch.epoch_ends && live_epoch.commit_datapoint_in_epoch;
            if epoch_is_over {
                Ok(Some(PoolCommand::Refresh))
            } else {
                Ok(None)
            }
        }
    }
}
