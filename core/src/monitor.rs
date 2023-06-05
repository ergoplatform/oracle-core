use std::sync::Arc;

use crate::box_kind::PoolBox;
use crate::oracle_state::OraclePool;
use crate::oracle_types::BlockHeight;
use crate::oracle_types::EpochLength;
use crate::pool_config::POOL_CONFIG;

#[derive(Debug, serde::Serialize)]
pub enum PoolStatus {
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
    pub status: PoolStatus,
    pub details: PoolHealthDetails,
}

pub fn check_pool_health(
    oracle_pool: Arc<OraclePool>,
    current_height: BlockHeight,
) -> Result<PoolHealth, anyhow::Error> {
    let pool_conf = &POOL_CONFIG;
    let pool_box_height = oracle_pool
        .get_pool_box_source()
        .get_pool_box()?
        .get_box()
        .creation_height
        .into();
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
            PoolStatus::Ok
        } else {
            PoolStatus::Down
        },
        details: PoolHealthDetails {
            pool_box_height,
            current_height,
            epoch_length,
        },
    })
}
