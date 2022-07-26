// This files relates to the state of the oracle/oracle pool.
use crate::box_kind::{
    BallotBoxError, BallotBoxWrapper, BallotBoxWrapperInputs, OracleBox, OracleBoxError,
    OracleBoxWrapper, OracleBoxWrapperInputs, PoolBox, PoolBoxError, PoolBoxWrapper,
    PoolBoxWrapperInputs, RefreshBoxError, RefreshBoxWrapper, RefreshBoxWrapperInputs,
};
use crate::contracts::ballot::BallotContract;
use crate::contracts::oracle::OracleContract;
use crate::datapoint_source::{DataPointSource, DataPointSourceError};
use crate::oracle_config::ORACLE_CONFIG;
use crate::scans::{
    register_datapoint_scan, register_local_ballot_box_scan, register_local_oracle_datapoint_scan,
    register_pool_box_scan, register_refresh_box_scan, save_scan_ids_locally, Scan, ScanError,
};
use crate::state::PoolState;
use crate::{BlockHeight, EpochID, NanoErg};
use anyhow::Error;
use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_node_interface::node_interface::NodeError;
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

/// A `Stage` in the multi-stage smart contract protocol. Is defined here by it's contract address & it's scan_id
#[derive(Debug, Clone)]
pub struct Stage {
    pub contract_address: String,
    pub scan: Scan,
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
#[derive(Debug)]
pub struct OraclePool<'a> {
    pub data_point_source: Box<dyn DataPointSource>,
    /// Stages
    pub datapoint_stage: DatapointStage<'a>,
    local_oracle_datapoint_scan: Option<LocalOracleDatapointScan<'a>>,
    local_ballot_box_scan: Option<LocalBallotBoxScan<'a>>,
    pool_box_scan: PoolBoxScan<'a>,
    refresh_box_scan: RefreshBoxScan<'a>,
}

#[derive(Debug)]
pub struct DatapointStage<'a> {
    pub stage: Stage,
    oracle_box_wrapper_inputs: OracleBoxWrapperInputs<'a>,
}

#[derive(Debug)]
pub struct LocalOracleDatapointScan<'a> {
    scan: Scan,
    oracle_box_wrapper_inputs: OracleBoxWrapperInputs<'a>,
}

#[derive(Debug)]
pub struct LocalBallotBoxScan<'a> {
    scan: Scan,
    ballot_box_wrapper_inputs: BallotBoxWrapperInputs<'a>,
}

#[derive(Debug)]
pub struct PoolBoxScan<'a> {
    scan: Scan,
    pool_box_wrapper_inputs: PoolBoxWrapperInputs<'a>,
}

#[derive(Debug)]
pub struct RefreshBoxScan<'a> {
    scan: Scan,
    refresh_box_wrapper_inputs: RefreshBoxWrapperInputs<'a>,
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

impl<'a> OraclePool<'a> {
    /// Create a new `OraclePool` struct
    pub fn new() -> std::result::Result<OraclePool<'a>, Error> {
        let config = &ORACLE_CONFIG;
        let local_oracle_address = config.oracle_address.clone();
        let oracle_pool_participant_token_id = config.token_ids.oracle_token_id.clone();
        let data_point_source = config.data_point_source()?;

        let refresh_box_scan_name = "Refresh Box Scan";

        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::from((&config.oracle_contract_parameters, &config.token_ids));
        let datapoint_contract_address =
            OracleContract::new(oracle_box_wrapper_inputs.into())?.ergo_tree();

        let ballot_box_wrapper_inputs = BallotBoxWrapperInputs {
            parameters: &config.ballot_parameters,
            ballot_token_id: &config.token_ids.ballot_token_id,
            update_nft_token_id: &config.token_ids.update_nft_token_id,
        };
        let pool_box_wrapper_inputs = PoolBoxWrapperInputs {
            contract_parameters: &config.pool_contract_parameters,
            pool_nft_token_id: &config.token_ids.pool_nft_token_id,
            reward_token_id: &config.token_ids.reward_token_id,
            refresh_nft_token_id: &config.token_ids.refresh_nft_token_id,
            update_nft_token_id: &config.token_ids.update_nft_token_id,
        };
        let refresh_box_wrapper_inputs = RefreshBoxWrapperInputs {
            contract_parameters: &config.refresh_contract_parameters,
            refresh_nft_token_id: &config.token_ids.refresh_nft_token_id,
            oracle_token_id: &config.token_ids.oracle_token_id,
            pool_nft_token_id: &config.token_ids.pool_nft_token_id,
        };

        // If scanIDs.json exists, skip registering scans & saving generated ids
        if !Path::new("scanIDs.json").exists() {
            let mut scans = vec![
                register_datapoint_scan(
                    &oracle_pool_participant_token_id,
                    &datapoint_contract_address,
                )
                .unwrap(),
                register_pool_box_scan(pool_box_wrapper_inputs).unwrap(),
                register_refresh_box_scan(refresh_box_scan_name, refresh_box_wrapper_inputs)
                    .unwrap(),
            ];

            // Local datapoint box may not exist yet.
            if let Ok(local_scan) = register_local_oracle_datapoint_scan(
                &oracle_pool_participant_token_id,
                &datapoint_contract_address,
                &local_oracle_address,
            ) {
                scans.push(local_scan);
            }

            let ballot_contract_address =
                BallotContract::new(ballot_box_wrapper_inputs.into())?.ergo_tree();
            // Local ballot box may not exist yet.
            if let Ok(local_scan) = register_local_ballot_box_scan(
                &ballot_contract_address,
                &config.token_ids.ballot_token_id,
                &config.ballot_parameters.ballot_token_owner_address,
            ) {
                scans.push(local_scan);
            }

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
            local_oracle_datapoint_scan = Some(LocalOracleDatapointScan {
                scan: Scan::new(
                    "Local Oracle Datapoint Scan",
                    &scan_json[local_scan_str].to_string(),
                ),
                oracle_box_wrapper_inputs,
            });
        };

        let local_scan_str = "Local Ballot Box Scan";
        let mut local_ballot_box_scan = None;
        if scan_json.has_key(local_scan_str) {
            local_ballot_box_scan = Some(LocalBallotBoxScan {
                scan: Scan::new(local_scan_str, &scan_json[local_scan_str].to_string()),
                ballot_box_wrapper_inputs,
            });
        }

        let pool_box_scan = PoolBoxScan {
            scan: Scan::new("Pool Box Scan", &scan_json["Pool Box Scan"].to_string()),
            pool_box_wrapper_inputs,
        };

        let refresh_box_scan = RefreshBoxScan {
            scan: Scan::new(
                refresh_box_scan_name,
                &scan_json[refresh_box_scan_name].to_string(),
            ),
            refresh_box_wrapper_inputs,
        };

        // Create `OraclePool` struct
        Ok(OraclePool {
            data_point_source,
            datapoint_stage: DatapointStage {
                stage: Stage {
                    contract_address: datapoint_contract_address.to_base16_bytes().unwrap(),
                    scan: datapoint_scan,
                },
                oracle_box_wrapper_inputs,
            },
            local_oracle_datapoint_scan,
            local_ballot_box_scan,
            pool_box_scan,
            refresh_box_scan,
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
        let epoch_ends = pool_box.get_box().creation_height
            + ORACLE_CONFIG.refresh_contract_parameters.epoch_length as u32;

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
}

impl<'a> PoolBoxSource for PoolBoxScan<'a> {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper> {
        let box_wrapper = PoolBoxWrapper::new(self.scan.get_box()?, self.pool_box_wrapper_inputs)?;
        Ok(box_wrapper)
    }
}

impl<'a> LocalBallotBoxSource for LocalBallotBoxScan<'a> {
    fn get_ballot_box(&self) -> Result<BallotBoxWrapper> {
        let box_wrapper =
            BallotBoxWrapper::new(self.scan.get_box()?, self.ballot_box_wrapper_inputs)?;
        Ok(box_wrapper)
    }
}

impl<'a> RefreshBoxSource for RefreshBoxScan<'a> {
    fn get_refresh_box(&self) -> Result<RefreshBoxWrapper> {
        let box_wrapper =
            RefreshBoxWrapper::new(self.scan.get_box()?, self.refresh_box_wrapper_inputs)?;
        Ok(box_wrapper)
    }
}

impl<'a> LocalDatapointBoxSource for LocalOracleDatapointScan<'a> {
    fn get_local_oracle_datapoint_box(&self) -> Result<OracleBoxWrapper> {
        let box_wrapper =
            OracleBoxWrapper::new(self.scan.get_box()?, self.oracle_box_wrapper_inputs)?;
        Ok(box_wrapper)
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

impl<'a> DatapointBoxesSource for DatapointStage<'a> {
    fn get_oracle_datapoint_boxes(&self) -> Result<Vec<OracleBoxWrapper>> {
        let res = self
            .stage
            .get_boxes()?
            .into_iter()
            .map(|b| OracleBoxWrapper::new(b, self.oracle_box_wrapper_inputs).unwrap())
            .collect();
        Ok(res)
    }
}
