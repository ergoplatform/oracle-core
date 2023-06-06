use std::sync::Arc;

use crate::box_kind::OracleBoxWrapper;
use crate::oracle_state::OraclePool;
use crate::oracle_types::BlockHeight;
use crate::oracle_types::EpochLength;
use crate::pool_config::POOL_CONFIG;

#[derive(Debug, serde::Serialize)]
pub enum HealthStatus {
    Ok,
    Down,
}

#[derive(Debug, serde::Serialize)]
pub struct PoolHealthDetails {
    pub pool_box_height: BlockHeight,
    pub current_height: BlockHeight,
    pub epoch_length: EpochLength,
}

#[derive(Debug, serde::Serialize)]
pub struct PoolHealth {
    pub status: HealthStatus,
    pub details: PoolHealthDetails,
}

pub fn check_pool_health(
    current_height: BlockHeight,
    pool_box_height: BlockHeight,
) -> Result<PoolHealth, anyhow::Error> {
    let pool_conf = &POOL_CONFIG;
    let epoch_length = pool_conf
        .refresh_box_wrapper_inputs
        .contract_inputs
        .contract_parameters()
        .epoch_length()
        .0
        .into();
    let acceptable_pool_box_delay_blocks = 3;
    let is_healthy =
        pool_box_height >= current_height - epoch_length - acceptable_pool_box_delay_blocks;
    Ok(PoolHealth {
        status: if is_healthy {
            HealthStatus::Ok
        } else {
            HealthStatus::Down
        },
        details: PoolHealthDetails {
            pool_box_height,
            current_height,
            epoch_length,
        },
    })
}

#[derive(Debug, serde::Serialize)]
pub struct OracleHealth {
    pub status: HealthStatus,
    pub details: OracleHealthDetails,
}

#[derive(Debug, serde::Serialize)]
pub enum OracleBoxDetails {
    PostedBox(BlockHeight),
    CollectedBox(BlockHeight),
}

impl OracleBoxDetails {
    pub fn oracle_box_height(&self) -> BlockHeight {
        match self {
            OracleBoxDetails::PostedBox(height) => *height,
            OracleBoxDetails::CollectedBox(height) => *height,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct OracleHealthDetails {
    pub pool_box_height: BlockHeight,
    pub box_details: OracleBoxDetails,
}

pub fn check_oracle_health(
    oracle_pool: Arc<OraclePool>,
    pool_box_height: BlockHeight,
) -> Result<OracleHealth, anyhow::Error> {
    let health = match oracle_pool
        .get_local_datapoint_box_source()
        .get_local_oracle_datapoint_box()?
        .ok_or_else(|| anyhow::anyhow!("Oracle box not found"))?
    {
        OracleBoxWrapper::Posted(posted_box) => {
            let posted_box_height = posted_box.get_box().creation_height.into();
            OracleHealth {
                status: if posted_box_height > pool_box_height {
                    HealthStatus::Ok
                } else {
                    HealthStatus::Down
                },
                details: OracleHealthDetails {
                    pool_box_height,
                    box_details: OracleBoxDetails::PostedBox(posted_box_height),
                },
            }
        }
        OracleBoxWrapper::Collected(collected_box) => {
            let creation_height = collected_box.get_box().creation_height.into();
            OracleHealth {
                status: if creation_height == pool_box_height {
                    HealthStatus::Ok
                } else {
                    HealthStatus::Down
                },
                details: OracleHealthDetails {
                    pool_box_height,
                    box_details: OracleBoxDetails::PostedBox(creation_height),
                },
            }
        }
    };
    Ok(health)
}
