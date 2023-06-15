use std::sync::Arc;

use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;

use crate::box_kind::CollectedOracleBox;
use crate::box_kind::OracleBoxWrapper;
use crate::box_kind::PostedOracleBox;
use crate::oracle_state::DataSourceError;
use crate::oracle_state::OraclePool;
use crate::oracle_types::BlockHeight;
use crate::oracle_types::EpochLength;
use crate::oracle_types::MinDatapoints;
use crate::pool_config::POOL_CONFIG;

#[derive(Debug, serde::Serialize, Copy, Clone)]
pub enum HealthStatus {
    Ok = 1,
    Down = 0,
}

impl HealthStatus {
    pub fn get_integer_value(&self) -> i32 {
        *self as i32
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PoolHealthDetails {
    pub pool_box_height: BlockHeight,
    pub current_height: BlockHeight,
    pub epoch_length: EpochLength,
    pub all_oracle_boxes: Vec<OracleDetails>,
    pub active_oracle_boxes: Vec<OracleDetails>,
    pub min_data_points: MinDatapoints,
    pub total_oracle_token_count: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OracleDetails {
    pub address: NetworkAddress,
    pub box_height: OracleBoxDetails,
}

#[derive(Debug, serde::Serialize)]
pub struct PoolHealth {
    pub status: HealthStatus,
    pub details: PoolHealthDetails,
}

pub fn check_pool_health(
    current_height: BlockHeight,
    pool_box_height: BlockHeight,
    oracle_pool: Arc<OraclePool>,
    network_prefix: NetworkPrefix,
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
    let total_oracle_token_count = oracle_pool.get_total_oracle_token_count()?;
    let all_oracles = get_all_oracle_boxes(oracle_pool, network_prefix)?;
    let active_oracles = get_active_oracle_boxes(&all_oracles, pool_box_height);
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
            all_oracle_boxes: all_oracles,
            active_oracle_boxes: active_oracles,
            min_data_points: pool_conf
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .min_data_points(),
            total_oracle_token_count,
        },
    })
}

pub fn get_all_oracle_boxes(
    oracle_pool: Arc<OraclePool>,
    network_prefix: NetworkPrefix,
) -> Result<Vec<OracleDetails>, DataSourceError> {
    let mut oracle_details = vec![];
    let posted_boxes = oracle_pool
        .get_posted_datapoint_boxes_source()
        .get_posted_datapoint_boxes()?;
    let collected_boxes = oracle_pool
        .get_collected_datapoint_boxes_source()
        .get_collected_datapoint_boxes()?;
    for b in posted_boxes {
        let detail = OracleDetails {
            address: NetworkAddress::new(network_prefix, &Address::P2Pk(b.public_key().into())),
            box_height: b.into(),
        };
        oracle_details.push(detail);
    }
    for b in collected_boxes {
        let detail = OracleDetails {
            address: NetworkAddress::new(network_prefix, &Address::P2Pk(b.public_key().into())),
            box_height: b.into(),
        };
        oracle_details.push(detail);
    }
    Ok(oracle_details)
}

pub fn get_active_oracle_boxes(
    all_oracle_boxes: &Vec<OracleDetails>,
    pool_box_height: BlockHeight,
) -> Vec<OracleDetails> {
    let mut active_oracles: Vec<OracleDetails> = vec![];
    for oracle_box in all_oracle_boxes {
        match oracle_box.box_height {
            OracleBoxDetails::PostedBox(posted_box_height) => {
                if posted_box_height >= pool_box_height {
                    active_oracles.push(oracle_box.clone());
                }
            }
            OracleBoxDetails::CollectedBox(collected_box_height) => {
                if collected_box_height == pool_box_height {
                    active_oracles.push(oracle_box.clone());
                }
            }
        }
    }
    active_oracles
}

#[derive(Debug, serde::Serialize)]
pub struct OracleHealth {
    pub status: HealthStatus,
    pub details: OracleHealthDetails,
}

#[derive(Debug, Clone, serde::Serialize)]
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

    pub fn label_name(&self) -> &'static str {
        match self {
            OracleBoxDetails::PostedBox(_) => "posted",
            OracleBoxDetails::CollectedBox(_) => "collected",
        }
    }
}

impl From<PostedOracleBox> for OracleBoxDetails {
    fn from(box_wrapper: PostedOracleBox) -> Self {
        OracleBoxDetails::PostedBox(box_wrapper.get_box().creation_height.into())
    }
}

impl From<CollectedOracleBox> for OracleBoxDetails {
    fn from(box_wrapper: CollectedOracleBox) -> Self {
        OracleBoxDetails::CollectedBox(box_wrapper.get_box().creation_height.into())
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
                    box_details: OracleBoxDetails::CollectedBox(creation_height),
                },
            }
        }
    };
    Ok(health)
}
