// This files relates to the state of the oracle/oracle pool.
use crate::box_kind::{
    OracleBox, OracleBoxError, OracleBoxWrapper, PoolBox, PoolBoxError, PoolBoxWrapper,
    RefreshBoxError, RefreshBoxWrapper,
};
use crate::contracts::pool::PoolContract;
use crate::contracts::refresh::RefreshContract;
use crate::datapoint_source::DataPointSourceError;
use crate::oracle_config::get_config_yaml;
use crate::scans::{
    register_datapoint_scan, register_epoch_preparation_scan, register_local_oracle_datapoint_scan,
    register_pool_box_scan, register_pool_deposit_scan, register_refresh_box_scan,
    save_scan_ids_locally, Scan, ScanError,
};
use crate::state::PoolState;
use crate::{BlockHeight, EpochID, NanoErg, P2PKAddress, TokenID};
use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_node_interface::node_interface::NodeError;
use std::convert::TryInto;
use std::path::Path;
use thiserror::Error;
use yaml_rust::YamlLoader;

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
#[derive(Debug, Clone)]
pub struct OraclePool {
    /// Address of the local oracle running the oracle core
    pub local_oracle_address: P2PKAddress,
    pub on_mainnet: bool,
    /// Token IDs
    pub oracle_pool_nft: TokenID,
    pub oracle_pool_participant_token: TokenID,
    pub reward_token: TokenID,
    /// Stages
    pub epoch_preparation_stage: Stage,
    pub datapoint_stage: Stage,
    pub pool_deposit_stage: Stage,
    // Local Oracle Datapoint Scan
    pub local_oracle_datapoint_scan: Option<Scan>,
    pool_box_scan: Scan,
    refresh_box_scan: Scan,
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
    pub fn new() -> OraclePool {
        let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];

        let local_oracle_address = config["oracle_address"]
            .as_str()
            .expect("No oracle_address specified in config file.")
            .to_string();

        let on_mainnet = config["on_mainnet"]
            .as_bool()
            .expect("on_mainnet not specified in config file.");

        let oracle_pool_nft: String = RefreshContract::new().pool_nft_token_id().into();
        let refresh_nft: String = PoolContract::new().refresh_nft_token_id().into();

        let oracle_pool_participant_token = config["oracle_pool_participant_token"]
            .as_str()
            .expect("No oracle_pool_participant_token specified in config file.")
            .to_string();

        let reward_token = config["reward_token"]
            .as_str()
            .expect("No reward_token specified in config file.")
            .to_string();

        let epoch_preparation_contract_address = config["epoch_preparation_contract_address"]
            .as_str()
            .expect("No epoch_preparation_contract_address specified in config file.")
            .to_string();
        let datapoint_contract_address = config["datapoint_contract_address"]
            .as_str()
            .expect("No datapoint_contract_address specified in config file.")
            .to_string();
        let pool_deposit_contract_address = config["pool_deposit_contract_address"]
            .as_str()
            .expect("No pool_deposit_contract_address specified in config file.")
            .to_string();

        let refresh_box_scan_name = "Refresh Box Scan";

        // If scanIDs.json exists, skip registering scans & saving generated ids
        if !Path::new("scanIDs.json").exists() {
            let mut scans = vec![
                register_epoch_preparation_scan(
                    &oracle_pool_nft,
                    &epoch_preparation_contract_address,
                )
                .unwrap(),
                register_datapoint_scan(
                    &oracle_pool_participant_token,
                    &datapoint_contract_address,
                )
                .unwrap(),
                register_pool_deposit_scan(&pool_deposit_contract_address).unwrap(),
                register_pool_box_scan(&oracle_pool_nft).unwrap(),
                register_refresh_box_scan(refresh_box_scan_name, &refresh_nft).unwrap(),
            ];

            // Local datapoint box may not exist yet.
            if let Ok(local_scan) = register_local_oracle_datapoint_scan(
                &oracle_pool_participant_token,
                &datapoint_contract_address,
                &local_oracle_address,
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
        let epoch_preparation_scan = Scan::new(
            "Epoch Preparation Scan",
            &scan_json["Epoch Preparation Scan"].to_string(),
        );
        let datapoint_scan = Scan::new(
            "All Oracle Datapoints Scan",
            &scan_json["All Datapoints Scan"].to_string(),
        );
        let local_scan_str = "Local Oracle Datapoint Scan";
        let mut local_oracle_datapoint_scan = None;
        if scan_json.has_key(local_scan_str) {
            local_oracle_datapoint_scan = Some(Scan::new(
                "Local Oracle Datapoint Scan",
                &scan_json[local_scan_str].to_string(),
            ));
        };
        let pool_deposit_scan = Scan::new(
            "Pool Deposits Scan",
            &scan_json["Pool Deposits Scan"].to_string(),
        );

        let pool_box_scan = Scan::new("Pool Box Scan", &scan_json["Pool Box Scan"].to_string());

        let refresh_box_scan = Scan::new(
            refresh_box_scan_name,
            &scan_json[refresh_box_scan_name].to_string(),
        );

        // Create `OraclePool` struct
        OraclePool {
            local_oracle_address,
            on_mainnet,
            oracle_pool_nft,
            oracle_pool_participant_token,
            reward_token,
            epoch_preparation_stage: Stage {
                contract_address: epoch_preparation_contract_address,
                scan: epoch_preparation_scan,
            },
            datapoint_stage: Stage {
                contract_address: datapoint_contract_address.clone(),
                scan: datapoint_scan,
            },
            pool_deposit_stage: Stage {
                contract_address: pool_deposit_contract_address,
                scan: pool_deposit_scan,
            },
            local_oracle_datapoint_scan,
            pool_box_scan,
            refresh_box_scan,
        }
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
        let epoch_ends = pool_box.get_box().creation_height + RefreshContract::new().epoch_length();

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
    pub fn get_pool_deposits_state(&self) -> Result<PoolDepositsState> {
        let deposits_box_list = self.pool_deposit_stage.get_boxes()?;

        // Sum up all Ergs held in pool deposit boxes
        let sum_ergs = deposits_box_list
            .iter()
            .fold(0, |acc, b| acc + *b.value.as_u64());

        let deposits_state = PoolDepositsState {
            number_of_boxes: deposits_box_list.len() as u64,
            total_nanoergs: sum_ergs,
        };

        Ok(deposits_state)
    }

    pub fn get_pool_box_source(&self) -> &dyn PoolBoxSource {
        &self.pool_box_scan as &dyn PoolBoxSource
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

impl PoolBoxSource for Scan {
    fn get_pool_box(&self) -> Result<PoolBoxWrapper> {
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
