use derive_more::From;
use ergo_lib::ergo_chain_types::DigestNError;
use ergo_lib::ergotree_ir::chain::address::{Address, AddressEncoderError};
use thiserror::Error;

use crate::actions::PoolAction;
use crate::box_kind::PoolBox;
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_state::{OraclePool, StageError};
use crate::pool_config::POOL_CONFIG;
use crate::wallet::WalletDataSource;

use self::publish_datapoint::build_publish_first_datapoint_action;
use self::publish_datapoint::{
    build_subsequent_publish_datapoint_action, PublishDatapointActionError,
};
use self::refresh::build_refresh_action;
use self::refresh::RefreshActionError;

pub mod publish_datapoint;
pub mod refresh;
#[cfg(test)]
pub(crate) mod test_utils;

#[derive(Debug)]
pub enum PoolCommand {
    Refresh,
    PublishFirstDataPoint,
    PublishSubsequentDataPoint { republish: bool },
}

#[derive(Debug, From, Error)]
pub enum PoolCommandError {
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("unexpected error: {0}")]
    Unexpected(String),
    #[error("error on building RefreshAction: {0}")]
    RefreshActionError(RefreshActionError),
    #[error("error on building PublishDatapointAction: {0}")]
    PublishDatapointActionError(PublishDatapointActionError),
    #[error("Digest error: {0}")]
    Digest(DigestNError),
    #[error("Address encoder error: {0}")]
    AddressEncoder(AddressEncoderError),
    #[error("Wrong oracle address type")]
    WrongOracleAddressType,
}

pub fn build_action(
    cmd: PoolCommand,
    op: &OraclePool,
    wallet: &dyn WalletDataSource,
    height: u32,
    change_address: Address,
) -> Result<PoolAction, PoolCommandError> {
    let refresh_box_source = op.get_refresh_box_source();
    let datapoint_stage_src = op.get_datapoint_boxes_source();
    let pool_box = op.get_pool_box_source().get_pool_box()?;
    let current_epoch_counter = pool_box.epoch_counter();
    let oracle_public_key =
        if let Address::P2Pk(public_key) = ORACLE_CONFIG.oracle_address.address() {
            public_key
        } else {
            return Err(PoolCommandError::WrongOracleAddressType);
        };

    match cmd {
        PoolCommand::PublishFirstDataPoint => build_publish_first_datapoint_action(
            wallet,
            height,
            change_address,
            oracle_public_key,
            POOL_CONFIG.oracle_box_wrapper_inputs.clone(),
            &*op.data_point_source,
        )
        .map_err(Into::into)
        .map(Into::into),
        PoolCommand::PublishSubsequentDataPoint { republish: _ } => {
            if let Some(local_datapoint_box) = op
                .get_local_datapoint_box_source()
                .get_local_oracle_datapoint_box()?
            {
                let new_epoch_counter = current_epoch_counter;
                build_subsequent_publish_datapoint_action(
                    &local_datapoint_box,
                    wallet,
                    height,
                    change_address,
                    &*op.data_point_source,
                    new_epoch_counter,
                    pool_box.rate(),
                )
                .map_err(Into::into)
                .map(Into::into)
            } else {
                Err(PoolCommandError::Unexpected(
                    "{cmd} error: No local datapoint box found".to_string(),
                ))
            }
        }
        PoolCommand::Refresh => build_refresh_action(
            op.get_pool_box_source(),
            refresh_box_source,
            datapoint_stage_src,
            POOL_CONFIG
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .max_deviation_percent() as u32,
            POOL_CONFIG
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .min_data_points() as u32,
            wallet,
            height,
            change_address,
            oracle_public_key.h.as_ref(),
        )
        .map_err(Into::into)
        .map(Into::into),
    }
}
