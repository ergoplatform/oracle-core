use crate::oracle_state::LiveEpochState;
use crate::oracle_state::LocalDatapointState::Collected;
use crate::oracle_state::LocalDatapointState::Posted;
use crate::oracle_types::BlockHeight;
use crate::oracle_types::EpochLength;
use crate::pool_commands::PoolCommand;

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
    epoch_length: EpochLength,
    current_height: BlockHeight,
) -> Option<PoolCommand> {
    let epoch_length_u32 = match u32::try_from(epoch_length.0) {
        Ok(len) if len > 0 => len,
        _ => {
            log::error!("Invalid non-positive epoch length: {}", epoch_length.0);
            return None;
        }
    };
    let min_start_height = BlockHeight(current_height.0.saturating_sub(epoch_length_u32));
    match pool_state {
        PoolState::NeedsBootstrap => {
            log::warn!(
                "No oracle pool found, needs bootstrap or wait for bootstrap txs to be on-chain"
            );
            None
        }
        PoolState::LiveEpoch(live_epoch) => {
            log::debug!("Height {current_height}. Live epoch state: {live_epoch:?}");
            if let Some(local_datapoint_box_state) = live_epoch.local_datapoint_box_state {
                match local_datapoint_box_state {
                    Collected { height: _ } => {
                        // publish datapoint in the second part of the epoch
                        // to avoid some oracle boxes becoming stale on next refresh
                        // (datapoint posted on the first block of the epoch goes out of the epoch window too fast)
                        let elapsed = current_height
                            .0
                            .saturating_sub(live_epoch.latest_pool_box_height.0);
                        if elapsed % epoch_length_u32 > epoch_length_u32 / 2 {
                            Some(PoolCommand::PublishSubsequentDataPoint { republish: false })
                        } else {
                            None
                        }
                    }
                    Posted { epoch_id, height } => {
                        if height < min_start_height || epoch_id != live_epoch.pool_box_epoch_id {
                            Some(PoolCommand::PublishSubsequentDataPoint { republish: true })
                        } else if live_epoch.latest_pool_box_height < min_start_height
                            && epoch_id == live_epoch.pool_box_epoch_id
                        {
                            Some(PoolCommand::Refresh)
                        } else {
                            None
                        }
                    }
                }
            } else {
                // no local datapoint found
                Some(PoolCommand::PublishFirstDataPoint)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle_types::{EpochCounter, Rate};

    fn live_state(
        pool_box_epoch_id: u32,
        latest_pool_box_height: u32,
        local_datapoint_box_state: Option<crate::oracle_state::LocalDatapointState>,
    ) -> PoolState {
        PoolState::LiveEpoch(LiveEpochState {
            pool_box_epoch_id: EpochCounter(pool_box_epoch_id),
            local_datapoint_box_state,
            latest_pool_datapoint: Rate::from(100),
            latest_pool_box_height: BlockHeight(latest_pool_box_height),
        })
    }

    #[test]
    fn test_process_normal_transitions() {
        let epoch_len = EpochLength(30);

        assert!(process(PoolState::NeedsBootstrap, epoch_len, BlockHeight(100)).is_none());

        assert!(matches!(
            process(live_state(1, 100, None), epoch_len, BlockHeight(105)),
            Some(PoolCommand::PublishFirstDataPoint)
        ));

        // Collected in first half of epoch -> wait
        assert!(process(
            live_state(
                1,
                100,
                Some(Collected {
                    height: BlockHeight(100)
                })
            ),
            epoch_len,
            BlockHeight(115),
        )
        .is_none());

        // Collected in second half of epoch -> publish subsequent datapoint
        assert!(matches!(
            process(
                live_state(
                    1,
                    100,
                    Some(Collected {
                        height: BlockHeight(100)
                    })
                ),
                epoch_len,
                BlockHeight(116),
            ),
            Some(PoolCommand::PublishSubsequentDataPoint { republish: false })
        ));

        // Posted for old epoch -> republish
        assert!(matches!(
            process(
                live_state(
                    2,
                    120,
                    Some(Posted {
                        epoch_id: EpochCounter(1),
                        height: BlockHeight(115),
                    }),
                ),
                epoch_len,
                BlockHeight(125),
            ),
            Some(PoolCommand::PublishSubsequentDataPoint { republish: true })
        ));

        // Posted for current epoch and pool box older than epoch length -> refresh
        assert!(matches!(
            process(
                live_state(
                    2,
                    100,
                    Some(Posted {
                        epoch_id: EpochCounter(2),
                        height: BlockHeight(120),
                    }),
                ),
                epoch_len,
                BlockHeight(131),
            ),
            Some(PoolCommand::Refresh)
        ));
    }

    #[test]
    fn test_process_checked_height_and_epoch_length() {
        // current_height < latest_pool_box_height should not underflow
        assert!(process(
            live_state(
                1,
                200,
                Some(Collected {
                    height: BlockHeight(200)
                })
            ),
            EpochLength(30),
            BlockHeight(195),
        )
        .is_none());

        // current_height < epoch_length should not underflow
        assert!(process(
            live_state(
                1,
                5,
                Some(Posted {
                    epoch_id: EpochCounter(1),
                    height: BlockHeight(6),
                }),
            ),
            EpochLength(30),
            BlockHeight(10),
        )
        .is_none());

        // zero or negative epoch_length should return None without panicking
        assert!(process(
            live_state(
                1,
                100,
                Some(Collected {
                    height: BlockHeight(100)
                })
            ),
            EpochLength(0),
            BlockHeight(120),
        )
        .is_none());
        assert!(process(
            live_state(
                1,
                100,
                Some(Collected {
                    height: BlockHeight(100)
                })
            ),
            EpochLength(-10),
            BlockHeight(120),
        )
        .is_none());
    }
}
