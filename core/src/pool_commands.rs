use ergo_lib::ergo_chain_types::DigestNError;
use ergo_lib::ergotree_ir::chain::address::{Address, AddressEncoderError};
use thiserror::Error;

use crate::action_report::PoolActionReport;
use crate::actions::PoolAction;
use crate::box_kind::PoolBox;
use crate::datapoint_source::RuntimeDataPointSource;
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_state::{DataSourceError, OraclePool};
use crate::oracle_types::BlockHeight;
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

#[derive(Debug, Error)]
pub enum PoolCommandError {
    #[error("data source error: {0}")]
    DataSourceError(#[from] DataSourceError),
    #[error("unexpected error: {0}")]
    Unexpected(String),
    #[error("error on building RefreshAction: {0}")]
    RefreshActionError(#[from] RefreshActionError),
    #[error("error on building PublishDatapointAction: {0}")]
    PublishDatapointActionError(#[from] PublishDatapointActionError),
    #[error("Digest error: {0}")]
    Digest(#[from] DigestNError),
    #[error("Address encoder error: {0}")]
    AddressEncoder(#[from] AddressEncoderError),
    #[error("Wrong oracle address type")]
    WrongOracleAddressType,
}

pub fn build_action(
    cmd: PoolCommand,
    op: &OraclePool,
    wallet: &dyn WalletDataSource,
    height: BlockHeight,
    change_address: Address,
    datapoint_source: &RuntimeDataPointSource,
) -> Result<(PoolAction, PoolActionReport), PoolCommandError> {
    let refresh_box_source = op.get_refresh_box_source();
    let datapoint_boxes_source = op.get_posted_datapoint_boxes_source();
    let pool_box = op.get_pool_box_source().get_pool_box()?;
    let current_epoch_counter = pool_box.epoch_counter();
    let oracle_public_key =
        if let Address::P2Pk(public_key) = ORACLE_CONFIG.oracle_address.address() {
            *public_key.h
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
            datapoint_source,
        )
        .map_err(Into::into)
        .map(|(action, report)| (action.into(), report.into())),
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
                    datapoint_source,
                    new_epoch_counter,
                    &POOL_CONFIG.token_ids.reward_token_id,
                )
                .map_err(Into::into)
                .map(|(action, report)| (action.into(), report.into()))
            } else {
                Err(PoolCommandError::Unexpected(
                    "{cmd} error: No local datapoint box found".to_string(),
                ))
            }
        }
        PoolCommand::Refresh => build_refresh_action(
            op.get_pool_box_source(),
            refresh_box_source,
            datapoint_boxes_source,
            POOL_CONFIG
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .max_deviation_percent() as u32,
            POOL_CONFIG
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .min_data_points(),
            wallet,
            height,
            change_address,
            &oracle_public_key,
            op.get_buyback_box_source(),
            POOL_CONFIG.dev_reward_ergo_tree_bytes.clone(),
        )
        .map_err(Into::into)
        .map(|(action, report)| (action.into(), report.into())),
    }
}
