// This files relates to the state of the oracle/oracle pool.
use crate::box_kind::{
    BallotBoxError, BallotBoxWrapper, OracleBox, OracleBoxError, OracleBoxWrapper, PoolBox,
    PoolBoxError, PoolBoxWrapper, RefreshBoxError, RefreshBoxWrapper, UpdateBoxError,
    UpdateBoxWrapper,
};
use crate::contracts::ballot::BallotContract;
use crate::contracts::oracle::OracleContract;
use crate::contracts::update::UpdateContract;
use crate::datapoint_source::{DataPointSource, DataPointSourceError};
use crate::oracle_config::ORACLE_CONFIG;
use crate::scans::{
    register_ballot_box_scan, register_datapoint_scan, register_local_ballot_box_scan,
    register_local_oracle_datapoint_scan, register_pool_box_scan, register_refresh_box_scan,
    register_update_box_scan, save_scan_ids_locally, Scan, ScanError,
};
use crate::state::PoolState;
use crate::{BlockHeight, EpochID, NanoErg};
use anyhow::Error;
use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_node_interface::node_interface::NodeError;
use std::convert::TryInto;
use std::path::Path;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StageError>;

#[derive(Debug, From, Error)]
pub enum StageError {
    #[error("node error: {0}")]
    NodeError(NodeError),
    #[error("unexpected data error: {0}")]
    UnexpectedData(TryExtractFromError),
    #[error("scan error: {0}")]
    ScanError(ScanError),
    #[error("pool box error: {0}")]
    PoolBoxError(PoolBoxError),
    #[error("ballot box error: {0}")]
    BallotBoxError(BallotBoxError),
    #[error("refresh box error: {0}")]
    RefreshBoxError(RefreshBoxError),
    #[error("oracle box error: {0}")]
    OracleBoxError(OracleBoxError),
    #[error("datapoint source error: {0}")]
    DataPointSource(DataPointSourceError),
    #[error("update box error: {0}")]
    UpdateBoxError(UpdateBoxError),
}

pub trait StageDataSource {
    /// Returns all boxes held at the given stage based on the registered scan
    fn get_boxes(&self) -> Result<Vec<ErgoBox>>;

    /// Returns the first box found by the registered scan for a given `Stage`
    fn get_box(&self) -> Result<ErgoBox>;

    /// Returns all boxes held at the given stage based on the registered scan
    /// serialized and ready to be used as rawInputs
    fn get_serialized_boxes(&self) -> Result<Vec<String>>;

    /// Returns the first box found by the registered scan for a given `Stage`
    /// serialized and ready to be used as a rawInput
    fn get_serialized_box(&self) -> Result<String>;

    /// Returns the number of boxes held at the given stage based on the registered scan
    fn number_of_boxes(&self) -> Result<u64>;
}

pub trait PoolBoxSource {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper>;
}

pub trait LocalBallotBoxSource {
    fn get_ballot_box(&self) -> Result<BallotBoxWrapper>;
}

pub trait RefreshBoxSource {
    fn get_refresh_box(&self) -> Result<RefreshBoxWrapper>;
}

pub trait DatapointBoxesSource {
    fn get_oracle_datapoint_boxes(&self) -> Result<Vec<OracleBoxWrapper>>;
}

pub trait LocalDatapointBoxSource {
    fn get_local_oracle_datapoint_box(&self) -> Result<OracleBoxWrapper>;
}

pub trait BallotBoxesSource {
    fn get_ballot_boxes(&self) -> Result<Vec<BallotBoxWrapper>>;
}

pub trait UpdateBoxSource {
    fn get_update_box(&self) -> Result<UpdateBoxWrapper>;
}

/// A `Stage` in the multi-stage smart contract protocol. Is defined here by it's contract address & it's scan_id
#[derive(Debug, Clone)]
pub struct Stage {
    pub contract_address: String,
    pub scan: Scan,
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
#[derive(Debug)]
pub struct OraclePool {
    pub data_point_source: Box<dyn DataPointSource>,
    /// Stages
    pub datapoint_stage: Stage,
    // Local Oracle Datapoint Scan
    pub local_oracle_datapoint_scan: Option<Scan>,
    // Local ballot box Scan
    pub local_ballot_box_scan: Option<Scan>,
    pub ballot_box_scan: Scan,
    pool_box_scan: Scan,
    refresh_box_scan: Scan,
    update_box_scan: Scan,
}

/// The state of the oracle pool when it is in the Live Epoch stage
#[derive(Debug, Clone)]
pub struct LiveEpochState {
    pub epoch_id: u32,
    pub commit_datapoint_in_epoch: bool,
    pub epoch_ends: BlockHeight,
    pub latest_pool_datapoint: u64,
}

/// The state of the oracle pool when it is in the Epoch Preparation stage
#[derive(Debug, Clone)]
pub struct PreparationState {
    pub funds: NanoErg,
    pub next_epoch_ends: BlockHeight,
    pub latest_pool_datapoint: u64,
}

/// The state of the local oracle's Datapoint box
#[derive(Debug, Clone)]
pub struct DatapointState {
    pub datapoint: u64,
    /// epoch counter of the epoch which the datapoint was posted in/originates from
    pub origin_epoch_id: EpochID,
    /// Height that the datapoint was declared as being created
    pub creation_height: BlockHeight,
}

/// The current UTXO-set state of all of the Pool Deposit boxes
#[derive(Debug, Clone)]
pub struct PoolDepositsState {
    pub number_of_boxes: u64,
    pub total_nanoergs: NanoErg,
}

impl OraclePool {
    /// Create a new `OraclePool` struct
    pub fn new() -> std::result::Result<OraclePool, Error> {
        let config = &ORACLE_CONFIG;
        let local_oracle_address = config.oracle_address.clone();
        let oracle_pool_nft = config.oracle_pool_nft.clone();
        let refresh_nft = config.refresh_nft.clone();
        let update_nft = config.update_nft.clone();
        let oracle_pool_participant_token_id = config.oracle_pool_participant_token_id.clone();
        let data_point_source = config.data_point_source()?;

        let refresh_box_scan_name = "Refresh Box Scan";
        let datapoint_contract_address = OracleContract::new()
            .with_pool_nft_token_id(oracle_pool_nft.clone())
            .ergo_tree();

        // If scanIDs.json exists, skip registering scans & saving generated ids
        if !Path::new("scanIDs.json").exists() {
            let mut scans = vec![
                register_datapoint_scan(
                    &oracle_pool_participant_token_id,
                    &datapoint_contract_address,
                )
                .unwrap(),
                register_pool_box_scan(&oracle_pool_nft, &refresh_nft, &config.update_nft).unwrap(),
                register_refresh_box_scan(
                    refresh_box_scan_name,
                    &refresh_nft,
                    &oracle_pool_participant_token_id,
                    &oracle_pool_nft,
                )
                .unwrap(),
                register_update_box_scan(&update_nft).unwrap(),
            ];

            // Local datapoint box may not exist yet.
            if let Ok(local_scan) = register_local_oracle_datapoint_scan(
                &oracle_pool_participant_token_id,
                &datapoint_contract_address,
                &local_oracle_address,
            ) {
                scans.push(local_scan);
            }

            let ballot_contract_address = BallotContract::new()
                .with_min_storage_rent(config.ballot_box_min_storage_rent)
                .with_update_nft_token_id(config.update_nft.clone())
                .ergo_tree();
            // Local ballot box may not exist yet.
            if let Ok(local_scan) = register_local_ballot_box_scan(
                &ballot_contract_address,
                &config.ballot_token_id,
                &config.ballot_token_owner_address,
            ) {
                scans.push(local_scan);
            }
            scans.push(
                register_ballot_box_scan(&ballot_contract_address, &config.ballot_token_id)
                    .unwrap(),
            );

            let res = save_scan_ids_locally(scans);
            if res.is_ok() {
                // Congrats scans registered screen here
                print!("\x1B[2J\x1B[1;1H");
                println!("====================================================================");
                println!("UTXO-Set Scans Have Been Successfully Registered With The Ergo Node");
                println!("====================================================================");
                println!("Press Enter To Continue...");
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).ok();
            } else if let Err(e) = res {
                // Failed, post error
                panic!("{:?}", e);
            }
        }

        // Read scanIDs.json for scan ids
        let scan_json = json::parse(
            &std::fs::read_to_string("scanIDs.json").expect("Unable to read scanIDs.json"),
        )
        .expect("Failed to parse scanIDs.json");

        // Create all `Scan` structs for protocol
        let datapoint_scan = Scan::new(
            "All Oracle Datapoints Scan",
            &scan_json["All Datapoints Scan"].to_string(),
        );
        let local_scan_str = "Local Oracle Datapoint Scan";
        let mut local_oracle_datapoint_scan = None;
        if scan_json.has_key(local_scan_str) {
            local_oracle_datapoint_scan = Some(Scan::new(
                local_scan_str,
                &scan_json[local_scan_str].to_string(),
            ));
        };

        let local_scan_str = "Local Ballot Box Scan";
        let mut local_ballot_box_scan = None;
        if scan_json.has_key(local_scan_str) {
            local_ballot_box_scan = Some(Scan::new(
                local_scan_str,
                &scan_json[local_scan_str].to_string(),
            ));
        }

        let ballot_box_scan =
            Scan::new("Ballot Box Scan", &scan_json["Ballot Box Scan"].to_string());

        let pool_box_scan = Scan::new("Pool Box Scan", &scan_json["Pool Box Scan"].to_string());

        let refresh_box_scan = Scan::new(
            refresh_box_scan_name,
            &scan_json[refresh_box_scan_name].to_string(),
        );

        let update_box_scan =
            Scan::new("Update Box Scan", &scan_json["Update Box Scan"].to_string());

        // Create `OraclePool` struct
        Ok(OraclePool {
            data_point_source,
            datapoint_stage: Stage {
                contract_address: datapoint_contract_address.to_base16_bytes().unwrap(),
                scan: datapoint_scan,
            },
            local_oracle_datapoint_scan,
            local_ballot_box_scan,
            ballot_box_scan,
            pool_box_scan,
            refresh_box_scan,
            update_box_scan,
        })
    }

    /// Get the current stage of the oracle pool box. Returns either `Preparation` or `Epoch`.
    pub fn check_oracle_pool_stage(&self) -> PoolState {
        match self.get_live_epoch_state() {
            Ok(s) => PoolState::LiveEpoch(s),
            Err(_) => PoolState::NeedsBootstrap,
        }
    }

    /// Get the state of the current oracle pool epoch
    pub fn get_live_epoch_state(&self) -> Result<LiveEpochState> {
        let pool_box = self.get_pool_box_source().get_pool_box()?;
        let epoch_id: u32 = pool_box.epoch_counter();
        // let epoch_box_id: String = epoch_box.box_id().into();

        // Whether datapoint was commit in the current Live Epoch
        let commit_datapoint_in_epoch = if let Some(datapoint_state) = self.get_datapoint_state()? {
            epoch_id == datapoint_state.origin_epoch_id
        } else {
            false
        };

        let latest_pool_datapoint = pool_box.rate();

        // Block height epochs ends is held in R5 of the epoch box
        let epoch_ends = pool_box.get_box().creation_height + ORACLE_CONFIG.epoch_length as u32;

        let epoch_state = LiveEpochState {
            epoch_id,
            commit_datapoint_in_epoch,
            epoch_ends: epoch_ends as u64,
            latest_pool_datapoint: latest_pool_datapoint as u64,
        };

        Ok(epoch_state)
    }

    // /// Get the state of the current epoch preparation box
    // pub fn get_preparation_state(&self) -> Result<PreparationState> {
    // let epoch_prep_box = self.epoch_preparation_stage.get_box()?;
    // let epoch_prep_box_regs = epoch_prep_box.additional_registers.get_ordered_values();

    // // Latest pool datapoint is held in R4
    // let latest_pool_datapoint = unwrap_long(&epoch_prep_box_regs[0])?;

    // // Next epoch ends height held in R5
    // let next_epoch_ends = unwrap_int(&epoch_prep_box_regs[1])?;

    // let prep_state = PreparationState {
    //     funds: *epoch_prep_box.value.as_u64(),
    //     next_epoch_ends: next_epoch_ends as u64,
    //     latest_pool_datapoint: latest_pool_datapoint as u64,
    // };

    // Ok(prep_state)
    // }

    /// Get the current state of the local oracle's datapoint
    pub fn get_datapoint_state(&self) -> Result<Option<DatapointState>> {
        if let Some(local_box) = &self.local_oracle_datapoint_scan {
            let datapoint_box = local_box.get_local_oracle_datapoint_box()?;

            let origin_epoch_id = datapoint_box.epoch_counter();

            let datapoint = datapoint_box.rate();

            let datapoint_state = DatapointState {
                datapoint,
                origin_epoch_id,
                creation_height: datapoint_box.get_box().creation_height as u64,
            };

            Ok(Some(datapoint_state))
        } else {
            Ok(None)
        }
    }

    /// Get the current state of all of the pool deposit boxes
    // pub fn get_pool_deposits_state(&self) -> Result<PoolDepositsState> {
    //     let deposits_box_list = self.pool_deposit_stage.get_boxes()?;

    //     // Sum up all Ergs held in pool deposit boxes
    //     let sum_ergs = deposits_box_list
    //         .iter()
    //         .fold(0, |acc, b| acc + *b.value.as_u64());

    //     let deposits_state = PoolDepositsState {
    //         number_of_boxes: deposits_box_list.len() as u64,
    //         total_nanoergs: sum_ergs,
    //     };

    //     Ok(deposits_state)
    // }

    pub fn get_pool_box_source(&self) -> &dyn PoolBoxSource {
        &self.pool_box_scan as &dyn PoolBoxSource
    }

    pub fn get_local_ballot_box_source(&self) -> Option<&dyn LocalBallotBoxSource> {
        self.local_ballot_box_scan
            .as_ref()
            .map(|s| s as &dyn LocalBallotBoxSource)
    }

    pub fn get_ballot_boxes_source(&self) -> &dyn BallotBoxesSource {
        &self.ballot_box_scan as &dyn BallotBoxesSource
    }

    pub fn get_refresh_box_source(&self) -> &dyn RefreshBoxSource {
        &self.refresh_box_scan as &dyn RefreshBoxSource
    }

    pub fn get_datapoint_boxes_source(&self) -> &dyn DatapointBoxesSource {
        &self.datapoint_stage as &dyn DatapointBoxesSource
    }

    pub fn get_local_datapoint_box_source(&self) -> Option<&dyn LocalDatapointBoxSource> {
        self.local_oracle_datapoint_scan
            .as_ref()
            .map(|s| s as &dyn LocalDatapointBoxSource)
    }

    pub fn get_update_box_source(&self) -> &dyn UpdateBoxSource {
        &self.update_box_scan as &dyn UpdateBoxSource
    }
}

impl PoolBoxSource for Scan {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper> {
        Ok(self.get_box()?.try_into()?)
    }
}

impl LocalBallotBoxSource for Scan {
    fn get_ballot_box(&self) -> Result<BallotBoxWrapper> {
        Ok(self.get_box()?.try_into()?)
    }
}

impl RefreshBoxSource for Scan {
    fn get_refresh_box(&self) -> Result<RefreshBoxWrapper> {
        Ok(self.get_box()?.try_into()?)
    }
}

impl LocalDatapointBoxSource for Scan {
    fn get_local_oracle_datapoint_box(&self) -> Result<OracleBoxWrapper> {
        Ok(self.get_box()?.try_into()?)
    }
}

impl BallotBoxesSource for Scan {
    fn get_ballot_boxes(&self) -> Result<Vec<BallotBoxWrapper>> {
        self.get_boxes()?
            .into_iter()
            .map(|ballot_box| Ok(ballot_box.try_into()?))
            .collect()
    }
}
impl UpdateBoxSource for Scan {
    fn get_update_box(&self) -> Result<UpdateBoxWrapper> {
        Ok(self.get_box()?.try_into()?)
    }
}

impl StageDataSource for Stage {
    /// Returns all boxes held at the given stage based on the registered scan
    fn get_boxes(&self) -> Result<Vec<ErgoBox>> {
        self.scan.get_boxes().map_err(Into::into)
    }

    /// Returns the first box found by the registered scan for a given `Stage`
    fn get_box(&self) -> Result<ErgoBox> {
        self.scan.get_box().map_err(Into::into)
    }

    /// Returns all boxes held at the given stage based on the registered scan
    /// serialized and ready to be used as rawInputs
    fn get_serialized_boxes(&self) -> Result<Vec<String>> {
        self.scan.get_serialized_boxes().map_err(Into::into)
    }

    /// Returns the first box found by the registered scan for a given `Stage`
    /// serialized and ready to be used as a rawInput
    fn get_serialized_box(&self) -> Result<String> {
        self.scan.get_serialized_box().map_err(Into::into)
    }

    /// Returns the number of boxes held at the given stage based on the registered scan
    fn number_of_boxes(&self) -> Result<u64> {
        Ok(self.get_boxes()?.len() as u64)
    }
}

impl DatapointBoxesSource for Stage {
    fn get_oracle_datapoint_boxes(&self) -> Result<Vec<OracleBoxWrapper>> {
        let res = self
            .get_boxes()?
            .into_iter()
            .map(|b| OracleBoxWrapper::new(b).unwrap())
            .collect();
        Ok(res)
    }
}
