use crate::box_kind::{
    BallotBox, BallotBoxError, BallotBoxWrapper, BallotBoxWrapperInputs, BuybackBoxError,
    BuybackBoxWrapper, CollectedOracleBox, OracleBox, OracleBoxError, OracleBoxWrapper,
    OracleBoxWrapperInputs, PoolBox, PoolBoxError, PoolBoxWrapper, PoolBoxWrapperInputs,
    PostedOracleBox, RefreshBoxError, RefreshBoxWrapper, RefreshBoxWrapperInputs, UpdateBoxError,
    UpdateBoxWrapper, UpdateBoxWrapperInputs, VoteBallotBoxWrapper,
};
use crate::datapoint_source::DataPointSourceError;
use crate::get_boxes::{GenericTokenFetch, GetBoxes, GetBoxesError, TokenFetchRegistry};
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_types::{BlockHeight, EpochCounter, Rate};
use crate::pool_config::POOL_CONFIG;
use crate::spec_token::{
    BallotTokenId, BuybackTokenId, OracleTokenId, PoolTokenId, RefreshTokenId, RewardTokenId,
    TokenIdKind, UpdateTokenId,
};
use crate::util::get_token_count;
use anyhow::Error;

use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, DataSourceError>;

#[derive(Debug, Error)]
pub enum DataSourceError {
    #[error("unexpected data error: {0}")]
    UnexpectedData(#[from] TryExtractFromError),
    #[error("get boxes error: {0}")]
    GetBoxesError(#[from] GetBoxesError),
    #[error("pool box error: {0}")]
    PoolBoxError(#[from] PoolBoxError),
    #[error("pool box not found")]
    PoolBoxNotFoundError,
    #[error("ballot box error: {0}")]
    BallotBoxError(#[from] BallotBoxError),
    #[error("refresh box error: {0}")]
    RefreshBoxError(#[from] RefreshBoxError),
    #[error("refresh box not found")]
    RefreshBoxNotFoundError,
    #[error("oracle box error: {0}")]
    OracleBoxError(#[from] OracleBoxError),
    #[error("datapoint source error: {0}")]
    DataPointSource(#[from] DataPointSourceError),
    #[error("update box error: {0}")]
    UpdateBoxError(#[from] UpdateBoxError),
    #[error("update box not found")]
    UpdateBoxNotFoundError,
    #[error("buyback box error: {0}")]
    BuybackBoxError(#[from] BuybackBoxError),
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

pub trait PostedDatapointBoxesSource {
    fn get_posted_datapoint_boxes(&self) -> Result<Vec<PostedOracleBox>>;
}

pub trait CollectedDatapointBoxesSource {
    fn get_collected_datapoint_boxes(&self) -> Result<Vec<CollectedOracleBox>>;
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

pub trait BuybackBoxSource {
    fn get_buyback_box(&self) -> Result<Option<BuybackBoxWrapper>>;
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
#[derive(Debug)]
pub struct OraclePool {
    oracle_datapoint_fetch: OracleDatapointFetch,
    local_oracle_datapoint_fetch: LocalOracleDatapointFetch,
    local_ballot_box_fetch: LocalBallotBoxFetch,
    pool_box_fetch: PoolBoxFetch,
    refresh_box_fetch: RefreshBoxFetch,
    ballot_boxes_fetch: BallotBoxesFetch,
    update_box_fetch: UpdateBoxFetch,
    buyback_box_fetch: Option<BuybackBoxFetch>,
}

#[derive(Debug)]
pub struct OracleDatapointFetch {
    token_fetch: GenericTokenFetch<OracleTokenId>,
    oracle_box_wrapper_inputs: OracleBoxWrapperInputs,
}

#[derive(Debug)]
pub struct LocalOracleDatapointFetch {
    token_fetch: GenericTokenFetch<OracleTokenId>,
    oracle_box_wrapper_inputs: OracleBoxWrapperInputs,
    oracle_pk: ProveDlog,
}

#[derive(Debug)]
pub struct LocalBallotBoxFetch {
    token_fetch: GenericTokenFetch<BallotTokenId>,
    ballot_box_wrapper_inputs: BallotBoxWrapperInputs,
    ballot_token_owner_pk: ProveDlog,
}

#[derive(Debug)]
pub struct PoolBoxFetch {
    token_fetch: GenericTokenFetch<PoolTokenId>,
    pool_box_wrapper_inputs: PoolBoxWrapperInputs,
}

#[derive(Debug)]
pub struct RefreshBoxFetch {
    token_fetch: GenericTokenFetch<RefreshTokenId>,
    refresh_box_wrapper_inputs: RefreshBoxWrapperInputs,
}

#[derive(Debug)]
pub struct BallotBoxesFetch {
    token_fetch: GenericTokenFetch<BallotTokenId>,
    ballot_box_wrapper_inputs: BallotBoxWrapperInputs,
}

#[derive(Debug)]
pub struct UpdateBoxFetch {
    token_fetch: GenericTokenFetch<UpdateTokenId>,
    update_box_wrapper_inputs: UpdateBoxWrapperInputs,
}

#[derive(Debug)]
pub struct BuybackBoxFetch {
    token_fetch: GenericTokenFetch<BuybackTokenId>,
    reward_token_id: RewardTokenId,
}

/// The state of the oracle pool when it is in the Live Epoch stage
#[derive(Debug, Clone)]
pub struct LiveEpochState {
    pub pool_box_epoch_id: EpochCounter,
    pub local_datapoint_box_state: Option<LocalDatapointState>,
    pub latest_pool_datapoint: Rate,
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

impl OraclePool {
    pub fn new(
        token_fetch_registry: &TokenFetchRegistry,
    ) -> std::result::Result<OraclePool, Error> {
        let pool_config = &POOL_CONFIG;
        let oracle_config = &ORACLE_CONFIG;
        let oracle_pk = oracle_config.oracle_address_p2pk()?;

        // Create all tokens structs for protocol
        let oracle_datapoint_fetch = OracleDatapointFetch {
            token_fetch: token_fetch_registry.oracle_token_fetch.clone(),
            oracle_box_wrapper_inputs: pool_config.oracle_box_wrapper_inputs.clone(),
        };
        let local_oracle_datapoint_fetch = LocalOracleDatapointFetch {
            token_fetch: token_fetch_registry.oracle_token_fetch.clone(),
            oracle_box_wrapper_inputs: pool_config.oracle_box_wrapper_inputs.clone(),
            oracle_pk: oracle_pk.clone(),
        };

        let local_ballot_box_fetch = LocalBallotBoxFetch {
            token_fetch: token_fetch_registry.ballot_token_fetch.clone(),
            ballot_box_wrapper_inputs: pool_config.ballot_box_wrapper_inputs.clone(),
            ballot_token_owner_pk: oracle_pk.clone(),
        };

        let ballot_boxes_fetch = BallotBoxesFetch {
            token_fetch: token_fetch_registry.ballot_token_fetch.clone(),
            ballot_box_wrapper_inputs: pool_config.ballot_box_wrapper_inputs.clone(),
        };

        let pool_box_fetch = PoolBoxFetch {
            token_fetch: token_fetch_registry.pool_token_fetch.clone(),
            pool_box_wrapper_inputs: pool_config.pool_box_wrapper_inputs.clone(),
        };

        let refresh_box_fetch = RefreshBoxFetch {
            token_fetch: token_fetch_registry.refresh_token_fetch.clone(),
            refresh_box_wrapper_inputs: pool_config.refresh_box_wrapper_inputs.clone(),
        };

        let update_box_fetch = UpdateBoxFetch {
            token_fetch: token_fetch_registry.update_token_fetch.clone(),
            update_box_wrapper_inputs: pool_config.update_box_wrapper_inputs.clone(),
        };

        let buyback_box_fetch =
            token_fetch_registry
                .buyback_token_fetch
                .clone()
                .map(|token_fetch| BuybackBoxFetch {
                    token_fetch,
                    reward_token_id: pool_config.token_ids.reward_token_id.clone(),
                });

        log::debug!("Tokens loaded");

        Ok(OraclePool {
            oracle_datapoint_fetch,
            local_oracle_datapoint_fetch,
            local_ballot_box_fetch,
            ballot_boxes_fetch,
            pool_box_fetch,
            refresh_box_fetch,
            update_box_fetch,
            buyback_box_fetch,
        })
    }

    /// Create a new `OraclePool` struct with loaded get_boxes
    pub fn load() -> std::result::Result<OraclePool, Error> {
        let token_fetch_registry = TokenFetchRegistry::load()?;
        Self::new(&token_fetch_registry)
    }

    /// Get the state of the current oracle pool epoch
    pub fn get_live_epoch_state(&self) -> std::result::Result<LiveEpochState, anyhow::Error> {
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

        let latest_pool_datapoint = pool_box.rate();

        let epoch_state = LiveEpochState {
            pool_box_epoch_id: epoch_id,
            latest_pool_datapoint,
            latest_pool_box_height: BlockHeight(pool_box.get_box().creation_height),
            local_datapoint_box_state,
        };

        Ok(epoch_state)
    }

    pub fn get_pool_box_source(&self) -> &dyn PoolBoxSource {
        &self.pool_box_fetch as &dyn PoolBoxSource
    }

    pub fn get_local_ballot_box_source(&self) -> &dyn LocalBallotBoxSource {
        &self.local_ballot_box_fetch as &dyn LocalBallotBoxSource
    }

    pub fn get_ballot_boxes_source(&self) -> &dyn VoteBallotBoxesSource {
        &self.ballot_boxes_fetch as &dyn VoteBallotBoxesSource
    }

    pub fn get_refresh_box_source(&self) -> &dyn RefreshBoxSource {
        &self.refresh_box_fetch as &dyn RefreshBoxSource
    }

    pub fn get_posted_datapoint_boxes_source(&self) -> &dyn PostedDatapointBoxesSource {
        &self.oracle_datapoint_fetch as &dyn PostedDatapointBoxesSource
    }

    pub fn get_collected_datapoint_boxes_source(&self) -> &dyn CollectedDatapointBoxesSource {
        &self.oracle_datapoint_fetch as &dyn CollectedDatapointBoxesSource
    }

    pub fn get_local_datapoint_box_source(&self) -> &dyn LocalDatapointBoxSource {
        &self.local_oracle_datapoint_fetch as &dyn LocalDatapointBoxSource
    }

    pub fn get_update_box_source(&self) -> &dyn UpdateBoxSource {
        &self.update_box_fetch as &dyn UpdateBoxSource
    }

    pub fn get_buyback_box_source(&self) -> Option<&dyn BuybackBoxSource> {
        self.buyback_box_fetch
            .as_ref()
            .map(|b| b as &dyn BuybackBoxSource)
    }

    pub fn get_total_oracle_token_count(&self) -> Result<u64> {
        Ok(self
            .oracle_datapoint_fetch
            .token_fetch
            .get_boxes()?
            .into_iter()
            .map(|b| {
                get_token_count(
                    b,
                    self.oracle_datapoint_fetch
                        .oracle_box_wrapper_inputs
                        .oracle_token_id
                        .token_id(),
                )
            })
            .sum::<u64>())
    }
}

impl PoolBoxSource for PoolBoxFetch {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper> {
        let box_wrapper = PoolBoxWrapper::new(
            self.token_fetch
                .get_box()?
                .ok_or(DataSourceError::PoolBoxNotFoundError)?,
            &self.pool_box_wrapper_inputs,
        )?;
        Ok(box_wrapper)
    }
}

impl LocalBallotBoxSource for LocalBallotBoxFetch {
    fn get_ballot_box(&self) -> Result<Option<BallotBoxWrapper>> {
        Ok(self
            .token_fetch
            .get_boxes()?
            .into_iter()
            .filter_map(|b| BallotBoxWrapper::new(b, &self.ballot_box_wrapper_inputs).ok())
            .find(|b| b.ballot_token_owner() == *self.ballot_token_owner_pk.h))
    }
}

impl RefreshBoxSource for RefreshBoxFetch {
    fn get_refresh_box(&self) -> Result<RefreshBoxWrapper> {
        let box_wrapper = RefreshBoxWrapper::new(
            self.token_fetch
                .get_box()?
                .ok_or(DataSourceError::RefreshBoxNotFoundError)?,
            &self.refresh_box_wrapper_inputs,
        )?;
        Ok(box_wrapper)
    }
}

impl LocalDatapointBoxSource for LocalOracleDatapointFetch {
    fn get_local_oracle_datapoint_box(&self) -> Result<Option<OracleBoxWrapper>> {
        Ok(self
            .token_fetch
            .get_boxes()?
            .into_iter()
            .filter_map(|b| OracleBoxWrapper::new(b, &self.oracle_box_wrapper_inputs).ok())
            .find(|b| b.public_key() == *self.oracle_pk.h))
    }
}

impl VoteBallotBoxesSource for BallotBoxesFetch {
    fn get_ballot_boxes(&self) -> Result<Vec<VoteBallotBoxWrapper>> {
        Ok(self
            .token_fetch
            .get_boxes()?
            .into_iter()
            .filter_map(|ballot_box| {
                VoteBallotBoxWrapper::new(ballot_box, &self.ballot_box_wrapper_inputs).ok()
            })
            .collect())
    }
}

impl UpdateBoxSource for UpdateBoxFetch {
    fn get_update_box(&self) -> Result<UpdateBoxWrapper> {
        let box_wrapper = UpdateBoxWrapper::new(
            self.token_fetch
                .get_box()?
                .ok_or(DataSourceError::UpdateBoxNotFoundError)?,
            &self.update_box_wrapper_inputs,
        )?;
        Ok(box_wrapper)
    }
}

impl PostedDatapointBoxesSource for OracleDatapointFetch {
    fn get_posted_datapoint_boxes(&self) -> Result<Vec<PostedOracleBox>> {
        let posted_boxes = self
            .token_fetch
            .get_boxes()?
            .into_iter()
            .filter_map(|b| OracleBoxWrapper::new(b, &self.oracle_box_wrapper_inputs).ok())
            .filter_map(|b| match b {
                OracleBoxWrapper::Posted(p) => Some(p),
                OracleBoxWrapper::Collected(_) => None,
            })
            .collect();
        Ok(posted_boxes)
    }
}

impl CollectedDatapointBoxesSource for OracleDatapointFetch {
    fn get_collected_datapoint_boxes(&self) -> Result<Vec<CollectedOracleBox>> {
        let posted_boxes = self
            .token_fetch
            .get_boxes()?
            .into_iter()
            .filter_map(|b| OracleBoxWrapper::new(b, &self.oracle_box_wrapper_inputs).ok())
            .filter_map(|b| match b {
                OracleBoxWrapper::Posted(_) => None,
                OracleBoxWrapper::Collected(p) => Some(p),
            })
            .collect();
        Ok(posted_boxes)
    }
}

impl BuybackBoxSource for BuybackBoxFetch {
    fn get_buyback_box(&self) -> Result<Option<BuybackBoxWrapper>> {
        Ok(self
            .token_fetch
            .get_box()?
            .map(|ergo_box| BuybackBoxWrapper::new(ergo_box, self.reward_token_id.clone())))
    }
}
