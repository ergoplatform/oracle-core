use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::Address;
use thiserror::Error;

use crate::actions::PoolAction;
use crate::oracle_state::{OraclePool, StageError};
use crate::wallet::WalletDataSource;

use self::publish_datapoint::build_publish_datapoint_action;
use self::publish_datapoint::PublishDatapointActionError;
use self::refresh::build_refresh_action;
use self::refresh::RefrechActionError;

mod publish_datapoint;
mod refresh;
#[cfg(test)]
mod test_utils;

pub enum PoolCommand {
    Bootstrap,
    Refresh,
    PublishDataPoint(i64),
}

#[derive(Debug, From, Error)]
pub enum PoolCommandError {
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("box builder error: {0}")]
    Unexpected(String),
    #[error("error on building RefreshAction: {0}")]
    RefrechActionError(RefrechActionError),
    #[error("error on building PublishDatapointAction: {0}")]
    PublishDatapointActionError(PublishDatapointActionError),
}

pub fn build_action(
    cmd: PoolCommand,
    op: OraclePool,
    wallet: &dyn WalletDataSource,
    height: u32,
    change_address: Address,
) -> Result<PoolAction, PoolCommandError> {
    let pool_box_source = op.get_pool_box_source();
    let refresh_box_source = op.get_refresh_box_source();
    let datapoint_stage_src = op.get_datapoint_boxes_source();
    let local_datapoint_box_source = op.get_local_datapoint_box_source();
    match cmd {
        PoolCommand::Bootstrap => todo!(),
        PoolCommand::Refresh => build_refresh_action(
            pool_box_source,
            refresh_box_source,
            datapoint_stage_src,
            wallet,
            height,
            change_address,
        )
        .map_err(Into::into)
        .map(Into::into),
        PoolCommand::PublishDataPoint(new_datapoint) => build_publish_datapoint_action(
            pool_box_source,
            local_datapoint_box_source,
            wallet,
            height,
            change_address,
            new_datapoint,
        )
        .map_err(Into::into)
        .map(Into::into),
    }
}
