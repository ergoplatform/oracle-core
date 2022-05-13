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

/// Enum for the state that the oracle pool is currently in
#[derive(Debug, Clone)]
pub enum PoolState {
    NeedsBootstrap,
    LiveEpoch(LiveEpochState),
}

pub fn process(
    pool_state: PoolState,
    // op: OraclePool,
    // parameters: PoolParameters,
    datapoint_script_name: &str,
    height: u64,
) -> Result<Option<PoolCommand>, StageError> {
    match pool_state {
        PoolState::NeedsBootstrap => todo!(),
        PoolState::LiveEpoch(live_epoch) => {
            let epoch_is_over =
                height >= live_epoch.epoch_ends && live_epoch.commit_datapoint_in_epoch;
            if epoch_is_over {
                Ok(Some(PoolCommand::Refresh))
            } else if !live_epoch.commit_datapoint_in_epoch {
                // Poll for new datapoint
                let script_output = std::process::Command::new(datapoint_script_name).output()?;
                let datapoint_str = String::from_utf8(script_output.stdout)?;
                let datapoint: i64 = datapoint_str.parse()?;
                Ok(Some(PoolCommand::PublishDataPoint(datapoint)))
            } else {
                Ok(None)
            }
        }
    }
}
