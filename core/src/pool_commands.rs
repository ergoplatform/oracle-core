use derive_more::From;
use ergo_lib::ergo_chain_types::DigestNError;
use ergo_lib::ergotree_ir::chain::address::{
    Address, AddressEncoder, AddressEncoderError, NetworkPrefix,
};
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use thiserror::Error;

use crate::actions::PoolAction;
use crate::box_kind::OracleBoxWrapperInputs;
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_state::{LocalDatapointBoxSource, OraclePool, StageError};
use crate::wallet::WalletDataSource;

use self::publish_datapoint::build_publish_datapoint_action;
use self::publish_datapoint::PublishDatapointActionError;
use self::refresh::build_refresh_action;
use self::refresh::RefrechActionError;

mod publish_datapoint;
mod refresh;
#[cfg(test)]
pub(crate) mod test_utils;

pub enum PoolCommand {
    Refresh,
    PublishDataPoint,
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
    let pool_box_source = op.get_pool_box_source();
    let refresh_box_source = op.get_refresh_box_source();
    let datapoint_stage_src = op.get_datapoint_boxes_source();
    match cmd {
        PoolCommand::Refresh => build_refresh_action(
            pool_box_source,
            refresh_box_source,
            datapoint_stage_src,
            ORACLE_CONFIG
                .refresh_contract_parameters
                .max_deviation_percent as u32,
            ORACLE_CONFIG.refresh_contract_parameters.min_data_points as u32,
            wallet,
            height,
            change_address,
        )
        .map_err(Into::into)
        .map(Into::into),
        PoolCommand::PublishDataPoint => {
            let inputs = if let Some(local_datapoint_box_source) =
                op.get_local_datapoint_box_source()
            {
                PublishDataPointCommandInputs::LocalDataPointBoxExists(local_datapoint_box_source)
            } else {
                let address_encoder = if ORACLE_CONFIG.on_mainnet {
                    AddressEncoder::new(NetworkPrefix::Mainnet)
                } else {
                    AddressEncoder::new(NetworkPrefix::Testnet)
                };
                let address =
                    address_encoder.parse_address_from_str(&ORACLE_CONFIG.oracle_address)?;
                if let Address::P2Pk(public_key) = address {
                    let oracle_box_wrapper_inputs = OracleBoxWrapperInputs::from((
                        &ORACLE_CONFIG.oracle_contract_parameters,
                        &ORACLE_CONFIG.token_ids,
                    ));
                    PublishDataPointCommandInputs::FirstDataPoint {
                        public_key,
                        oracle_box_wrapper_inputs,
                    }
                } else {
                    return Err(PoolCommandError::WrongOracleAddressType);
                }
            };
            build_publish_datapoint_action(
                pool_box_source,
                inputs,
                wallet,
                &*op.data_point_source,
                height,
                change_address,
            )
            .map_err(Into::into)
            .map(Into::into)
        }
    }
}

pub enum PublishDataPointCommandInputs<'a> {
    /// Local datapoint box already exists so pass in the associated `LocalDatapoinBoxSource`
    /// instance
    LocalDataPointBoxExists(&'a dyn LocalDatapointBoxSource),
    /// The first datapoint will be submitted, so there doesn't exist a local datapoint box now.
    FirstDataPoint {
        oracle_box_wrapper_inputs: OracleBoxWrapperInputs<'a>,
        public_key: ProveDlog,
    },
}
