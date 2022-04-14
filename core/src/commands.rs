use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::Address;
use thiserror::Error;

use crate::actions::PoolAction;
use crate::oracle_state::DatapointBoxesSource;
use crate::oracle_state::PoolBoxSource;
use crate::oracle_state::RefreshBoxSource;
use crate::oracle_state::StageError;
use crate::wallet::WalletDataSource;

use self::refresh::build_refresh_action;
use self::refresh::RefrechActionError;

mod refresh;

pub enum PoolCommand {
    Bootstrap,
    Refresh,
}

#[derive(Debug, From, Error)]
pub enum PoolCommandError {
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("box builder error: {0}")]
    Unexpected(String),
    #[error("error on building RefreshAction: {0}")]
    RefrechActionError(RefrechActionError),
}

pub fn build_action<B: DatapointBoxesSource, C: WalletDataSource>(
    cmd: PoolCommand,
    pool_box_source: &dyn PoolBoxSource,
    refresh_box_source: &dyn RefreshBoxSource,
    datapoint_stage_src: B,
    wallet: C,
    height: u32,
    change_address: Address,
) -> Result<PoolAction, PoolCommandError> {
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
    }
}
