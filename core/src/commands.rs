use derive_more::From;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::wallet::box_selector::BoxSelectorError;
use ergo_lib::wallet::tx_builder::TxBuilderError;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::actions::PoolAction;
use crate::oracle_state::DatapointStage;
use crate::oracle_state::LiveEpochStage;
use crate::oracle_state::StageError;
use crate::wallet::WalletDataSource;

use self::refresh::build_refresh_action;

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
    ErgoBoxCandidateBuilderError(ErgoBoxCandidateBuilderError),
    #[error("tx builder error: {0}")]
    TxBuilderError(TxBuilderError),
    #[error("node error: {0}")]
    NodeError(NodeError),
    #[error("box selector error: {0}")]
    BoxSelectorError(BoxSelectorError),
    #[error("not enough oracle boxes error: found {found}, expected {expected}")]
    NotEnoughOracleBoxes { found: u32, expected: u32 },
    #[error("unexpected error: {0}")]
    Unexpected(String),
}

pub fn build_action<A: LiveEpochStage, B: DatapointStage, C: WalletDataSource>(
    cmd: PoolCommand,
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
    wallet: C,
    height: u32,
    change_address: Address,
) -> Result<PoolAction, PoolCommandError> {
    match cmd {
        PoolCommand::Bootstrap => todo!(),
        PoolCommand::Refresh => build_refresh_action(
            live_epoch_stage_src,
            datapoint_stage_src,
            wallet,
            height,
            change_address,
        )
        .map(Into::into),
    }
}
