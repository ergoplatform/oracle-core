use crate::box_kind::{
    BallotBox, BallotBoxError, BallotBoxWrapper, BallotBoxWrapperInputs, OracleBox, OracleBoxError,
    OracleBoxWrapper, OracleBoxWrapperInputs, PoolBox, PoolBoxError, PoolBoxWrapper,
    PoolBoxWrapperInputs, PostedOracleBox, RefreshBoxError, RefreshBoxWrapper,
    RefreshBoxWrapperInputs, UpdateBoxError, UpdateBoxWrapper, UpdateBoxWrapperInputs,
    VoteBallotBoxWrapper,
};
use crate::datapoint_source::DataPointSourceError;
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_types::{BlockHeight, EpochCounter};
use crate::pool_config::POOL_CONFIG;
use crate::scans::{GenericTokenScan, NodeScanRegistry, ScanError, ScanGetBoxes};
use crate::spec_token::{BallotTokenId, OracleTokenId, PoolTokenId, RefreshTokenId, UpdateTokenId};
use anyhow::Error;
use derive_more::From;

use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, DataSourceError>;

#[derive(Debug, From, Error)]
pub enum DataSourceError {
    #[error("unexpected data error: {0}")]
    UnexpectedData(TryExtractFromError),
    #[error("scan error: {0}")]
    ScanError(ScanError),
    #[error("pool box error: {0}")]
    PoolBoxError(PoolBoxError),
    #[error("pool box not found")]
    PoolBoxNotFoundError,
    #[error("ballot box error: {0}")]
    BallotBoxError(BallotBoxError),
    #[error("refresh box error: {0}")]
    RefreshBoxError(RefreshBoxError),
    #[error("refresh box not found")]
    RefreshBoxNotFoundError,
    #[error("oracle box error: {0}")]
    OracleBoxError(OracleBoxError),
    #[error("datapoint source error: {0}")]
    DataPointSource(DataPointSourceError),
    #[error("update box error: {0}")]
    UpdateBoxError(UpdateBoxError),
    #[error("update box not found")]
    UpdateBoxNotFoundError,
}

pub trait PoolBoxSource {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper>;
}

pub trait LocalBallotBoxSource {
    fn get_ballot_box(&self) -> Result<Option<BallotBoxWrapper>>;
}

pub trait RefreshBoxSource {
    fn get_refresh_box(&self) -> Result<RefreshBoxWrapper>;
}

pub trait DatapointBoxesSource {
    fn get_oracle_datapoint_boxes(&self) -> Result<Vec<PostedOracleBox>>;
}

pub trait LocalDatapointBoxSource {
    fn get_local_oracle_datapoint_box(&self) -> Result<Option<OracleBoxWrapper>>;
}

pub trait VoteBallotBoxesSource {
    fn get_ballot_boxes(&self) -> Result<Vec<VoteBallotBoxWrapper>>;
}

pub trait UpdateBoxSource {
    fn get_update_box(&self) -> Result<UpdateBoxWrapper>;
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
#[derive(Debug)]
pub struct OraclePool<'a> {
    oracle_datapoint_scan: OracleDatapointScan<'a>,
    local_oracle_datapoint_scan: LocalOracleDatapointScan<'a>,
    local_ballot_box_scan: LocalBallotBoxScan<'a>,
    pool_box_scan: PoolBoxScan<'a>,
    refresh_box_scan: RefreshBoxScan<'a>,
    ballot_boxes_scan: BallotBoxesScan<'a>,
    update_box_scan: UpdateBoxScan<'a>,
}

#[derive(Debug)]
pub struct OracleDatapointScan<'a> {
    scan: GenericTokenScan<OracleTokenId>,
    oracle_box_wrapper_inputs: &'a OracleBoxWrapperInputs,
}

#[derive(Debug)]
pub struct LocalOracleDatapointScan<'a> {
    scan: GenericTokenScan<OracleTokenId>,
    oracle_box_wrapper_inputs: &'a OracleBoxWrapperInputs,
    oracle_pk: ProveDlog,
}

#[derive(Debug)]
pub struct LocalBallotBoxScan<'a> {
    scan: GenericTokenScan<BallotTokenId>,
    ballot_box_wrapper_inputs: &'a BallotBoxWrapperInputs,
    ballot_token_owner_pk: ProveDlog,
}

#[derive(Debug)]
pub struct PoolBoxScan<'a> {
    scan: GenericTokenScan<PoolTokenId>,
    pool_box_wrapper_inputs: &'a PoolBoxWrapperInputs,
}

#[derive(Debug)]
pub struct RefreshBoxScan<'a> {
    scan: GenericTokenScan<RefreshTokenId>,
    refresh_box_wrapper_inputs: &'a RefreshBoxWrapperInputs,
}

#[derive(Debug)]
pub struct BallotBoxesScan<'a> {
    scan: GenericTokenScan<BallotTokenId>,
    ballot_box_wrapper_inputs: &'a BallotBoxWrapperInputs,
}
#[derive(Debug)]
pub struct UpdateBoxScan<'a> {
    scan: GenericTokenScan<UpdateTokenId>,
    update_box_wrapper_inputs: &'a UpdateBoxWrapperInputs,
}

/// The state of the oracle pool when it is in the Live Epoch stage
#[derive(Debug, Clone)]
pub struct LiveEpochState {
    pub pool_box_epoch_id: EpochCounter,
    pub local_datapoint_box_state: Option<LocalDatapointState>,
    pub latest_pool_datapoint: u64,
    pub latest_pool_box_height: BlockHeight,
}

/// Last posted datapoint box info by the local oracle
#[derive(Debug, Clone)]
pub enum LocalDatapointState {
    Collected {
        height: BlockHeight,
    },
    Posted {
        epoch_id: EpochCounter,
        height: BlockHeight,
    },
}

impl<'a> OraclePool<'a> {
    pub fn new(
        node_scan_registry: &NodeScanRegistry,
    ) -> std::result::Result<OraclePool<'static>, Error> {
        let pool_config = &POOL_CONFIG;
        let oracle_config = &ORACLE_CONFIG;
        let oracle_pk = oracle_config.oracle_address_p2pk()?;

        // Create all `Scan` structs for protocol
        let oracle_datapoint_scan = OracleDatapointScan {
            scan: node_scan_registry.oracle_token_scan.clone(),
            oracle_box_wrapper_inputs: &pool_config.oracle_box_wrapper_inputs,
        };
        let local_oracle_datapoint_scan = LocalOracleDatapointScan {
            scan: node_scan_registry.oracle_token_scan.clone(),
            oracle_box_wrapper_inputs: &pool_config.oracle_box_wrapper_inputs,
            oracle_pk: oracle_pk.clone(),
        };

        let local_ballot_box_scan = LocalBallotBoxScan {
            scan: node_scan_registry.ballot_token_scan.clone(),
            ballot_box_wrapper_inputs: &pool_config.ballot_box_wrapper_inputs,
            ballot_token_owner_pk: oracle_pk.clone(),
        };

        let ballot_boxes_scan = BallotBoxesScan {
            scan: node_scan_registry.ballot_token_scan.clone(),
            ballot_box_wrapper_inputs: &pool_config.ballot_box_wrapper_inputs,
        };

        let pool_box_scan = PoolBoxScan {
            scan: node_scan_registry.pool_token_scan.clone(),
            pool_box_wrapper_inputs: &pool_config.pool_box_wrapper_inputs,
        };

        let refresh_box_scan = RefreshBoxScan {
            scan: node_scan_registry.refresh_token_scan.clone(),
            refresh_box_wrapper_inputs: &pool_config.refresh_box_wrapper_inputs,
        };

        let update_box_scan = UpdateBoxScan {
            scan: node_scan_registry.update_token_scan.clone(),
            update_box_wrapper_inputs: &pool_config.update_box_wrapper_inputs,
        };

        log::debug!("Scans loaded");

        Ok(OraclePool {
            oracle_datapoint_scan,
            local_oracle_datapoint_scan,
            local_ballot_box_scan,
            ballot_boxes_scan,
            pool_box_scan,
            refresh_box_scan,
            update_box_scan,
        })
    }

    /// Create a new `OraclePool` struct with loaded scans
    pub fn load() -> std::result::Result<OraclePool<'static>, Error> {
        let node_scan_registry = NodeScanRegistry::load()?;
        Self::new(&node_scan_registry)
    }

    /// Get the state of the current oracle pool epoch
    pub fn get_live_epoch_state(&self) -> Result<LiveEpochState> {
        let pool_box = self.get_pool_box_source().get_pool_box()?;
        let epoch_id = pool_box.epoch_counter();

        // Whether datapoint was commit in the current Live Epoch
        let local_datapoint_box_state = self
            .get_local_datapoint_box_source()
            .get_local_oracle_datapoint_box()?
            .map(|local_data_point_box| match local_data_point_box {
                OracleBoxWrapper::Posted(ref posted_box) => LocalDatapointState::Posted {
                    epoch_id: posted_box.epoch_counter(),
                    height: BlockHeight(local_data_point_box.get_box().creation_height),
                },
                OracleBoxWrapper::Collected(_) => LocalDatapointState::Collected {
                    height: BlockHeight(local_data_point_box.get_box().creation_height),
                },
            });

        let latest_pool_datapoint = pool_box.rate() as u64;

        let epoch_state = LiveEpochState {
            pool_box_epoch_id: epoch_id,
            latest_pool_datapoint,
            latest_pool_box_height: BlockHeight(pool_box.get_box().creation_height),
            local_datapoint_box_state,
        };

        Ok(epoch_state)
    }

    pub fn get_pool_box_source(&self) -> &dyn PoolBoxSource {
        &self.pool_box_scan as &dyn PoolBoxSource
    }

    pub fn get_local_ballot_box_source(&self) -> &dyn LocalBallotBoxSource {
        &self.local_ballot_box_scan as &dyn LocalBallotBoxSource
    }

    pub fn get_ballot_boxes_source(&self) -> &dyn VoteBallotBoxesSource {
        &self.ballot_boxes_scan as &dyn VoteBallotBoxesSource
    }

    pub fn get_refresh_box_source(&self) -> &dyn RefreshBoxSource {
        &self.refresh_box_scan as &dyn RefreshBoxSource
    }

    pub fn get_datapoint_boxes_source(&self) -> &dyn DatapointBoxesSource {
        &self.oracle_datapoint_scan as &dyn DatapointBoxesSource
    }

    pub fn get_local_datapoint_box_source(&self) -> &dyn LocalDatapointBoxSource {
        &self.local_oracle_datapoint_scan as &dyn LocalDatapointBoxSource
    }

    pub fn get_update_box_source(&self) -> &dyn UpdateBoxSource {
        &self.update_box_scan as &dyn UpdateBoxSource
    }
}

impl<'a> PoolBoxSource for PoolBoxScan<'a> {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper> {
        let box_wrapper = PoolBoxWrapper::new(
            self.scan
                .get_box()?
                .ok_or(DataSourceError::PoolBoxNotFoundError)?,
            self.pool_box_wrapper_inputs,
        )?;
        Ok(box_wrapper)
    }
}

impl<'a> LocalBallotBoxSource for LocalBallotBoxScan<'a> {
    fn get_ballot_box(&self) -> Result<Option<BallotBoxWrapper>> {
        Ok(self
            .scan
            .get_boxes()?
            .into_iter()
            .filter_map(|b| BallotBoxWrapper::new(b, self.ballot_box_wrapper_inputs).ok())
            .find(|b| b.ballot_token_owner() == *self.ballot_token_owner_pk.h))
    }
}

impl<'a> RefreshBoxSource for RefreshBoxScan<'a> {
    fn get_refresh_box(&self) -> Result<RefreshBoxWrapper> {
        let box_wrapper = RefreshBoxWrapper::new(
            self.scan
                .get_box()?
                .ok_or(DataSourceError::RefreshBoxNotFoundError)?,
            self.refresh_box_wrapper_inputs,
        )?;
        Ok(box_wrapper)
    }
}

impl<'a> LocalDatapointBoxSource for LocalOracleDatapointScan<'a> {
    fn get_local_oracle_datapoint_box(&self) -> Result<Option<OracleBoxWrapper>> {
        Ok(self
            .scan
            .get_boxes()?
            .into_iter()
            .filter_map(|b| OracleBoxWrapper::new(b, self.oracle_box_wrapper_inputs).ok())
            .find(|b| b.public_key() == *self.oracle_pk.h))
    }
}

impl<'a> VoteBallotBoxesSource for BallotBoxesScan<'a> {
    fn get_ballot_boxes(&self) -> Result<Vec<VoteBallotBoxWrapper>> {
        Ok(self
            .scan
            .get_boxes()?
            .into_iter()
            .filter_map(|ballot_box| {
                VoteBallotBoxWrapper::new(ballot_box, self.ballot_box_wrapper_inputs).ok()
            })
            .collect())
    }
}

impl<'a> UpdateBoxSource for UpdateBoxScan<'a> {
    fn get_update_box(&self) -> Result<UpdateBoxWrapper> {
        let box_wrapper = UpdateBoxWrapper::new(
            self.scan
                .get_box()?
                .ok_or(DataSourceError::UpdateBoxNotFoundError)?,
            self.update_box_wrapper_inputs,
        )?;
        Ok(box_wrapper)
    }
}

impl<'a> DatapointBoxesSource for OracleDatapointScan<'a> {
    fn get_oracle_datapoint_boxes(&self) -> Result<Vec<PostedOracleBox>> {
        let posted_boxes = self
            .scan
            .get_boxes()?
            .into_iter()
            .filter_map(|b| OracleBoxWrapper::new(b, self.oracle_box_wrapper_inputs).ok())
            .filter_map(|b| match b {
                OracleBoxWrapper::Posted(p) => Some(p),
                OracleBoxWrapper::Collected(_) => None,
            })
            .collect();
        Ok(posted_boxes)
    }
}
