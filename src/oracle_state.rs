use crate::encoding::{deserialize_integer, deserialize_string};
/// This files relates to the state of the oracle/oracle pool.
use crate::node_interface::{get_scan_boxes, serialize_box, serialize_boxes};
use crate::oracle_config::get_config_yaml;
use crate::scans::{
    register_collection_scan, register_datapoint_scan, register_epoch_preparation_scan,
    register_live_epoch_scan, register_pool_deposit_scan, save_scan_ids_locally,
};
use crate::{BlockHeight, EpochID, NanoErg};
use sigma_tree::ast::{CollPrim, Constant, ConstantVal};
use sigma_tree::chain::{Base16EncodedBytes, ErgoBox, ErgoBoxCandidate};
use std::path::Path;
use yaml_rust::YamlLoader;

/// Enum for the state that the oracle pool box is currently in
#[derive(Debug, Clone)]
pub enum PoolBoxState {
    Preparation,
    LiveEpoch,
}

/// A `Stage` is defined here by it's contract address & it's scan_id
#[derive(Debug, Clone)]
pub struct Stage {
    pub contract_address: String,
    pub scan_id: String,
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
#[derive(Debug, Clone)]
pub struct OraclePool {
    /// Address of the local oracle running the oracle core
    pub local_oracle_address: String,
    /// Token IDs
    pub oracle_pool_nft: String,
    pub oracle_pool_participant_token: String,
    /// Stages
    pub epoch_preparation_stage: Stage,
    pub live_epoch_stage: Stage,
    pub datapoint_stage: Stage,
    pub pool_deposit_stage: Stage,
}

/// The state of the oracle pool when it is in the Live Epoch stage
#[derive(Debug, Clone)]
pub struct LiveEpochState {
    pub funds: NanoErg,
    pub epoch_id: EpochID,
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
    /// Box id of the epoch which the datapoint was posted in/originates from
    pub origin_epoch_id: EpochID,
}

/// The current UTXO-set state of all of the Pool Deposit boxes
#[derive(Debug, Clone)]
pub struct PoolDepositsState {
    pub number_of_boxes: u64,
    pub total_nanoergs: u64,
}

impl OraclePool {
    /// Create a new `OraclePool` struct
    pub fn new() -> OraclePool {
        let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];

        let local_oracle_address = config["oracle_address"]
            .as_str()
            .expect("No oracle_pool_nft specified in config file.")
            .to_string();
        let oracle_pool_nft = config["oracle_pool_nft"]
            .as_str()
            .expect("No oracle_pool_nft specified in config file.")
            .to_string();
        let oracle_pool_participant_token = config["oracle_pool_participant_token"]
            .as_str()
            .expect("No oracle_pool_participant_token specified in config file.")
            .to_string();

        let epoch_preparation_contract_address = config["epoch_preparation_contract_address"]
            .as_str()
            .expect("No epoch_preparation_contract_address specified in config file.")
            .to_string();
        let live_epoch_contract_address = config["live_epoch_contract_address"]
            .as_str()
            .expect("No live_epoch_contract_address specified in config file.")
            .to_string();
        let datapoint_contract_address = config["datapoint_contract_address"]
            .as_str()
            .expect("No datapoint_contract_address specified in config file.")
            .to_string();
        let pool_deposit_contract_address = config["pool_deposit_contract_address"]
            .as_str()
            .expect("No pool_deposit_contract_address specified in config file.")
            .to_string();

        // If scanIDs.json exists, skip registering scans & saving generated ids
        if !Path::new("scanIDs.json").exists() {
            let id1 = register_epoch_preparation_scan(
                &oracle_pool_nft,
                &epoch_preparation_contract_address,
            );
            let id2 = register_live_epoch_scan(&oracle_pool_nft, &live_epoch_contract_address);
            let id3 = register_datapoint_scan(
                &oracle_pool_participant_token,
                &datapoint_contract_address,
                &local_oracle_address,
            );
            let id4 = register_pool_deposit_scan(&pool_deposit_contract_address);
            let id5 = register_collection_scan(
                &oracle_pool_participant_token,
                &datapoint_contract_address,
            );

            save_scan_ids_locally(id1, id2, id3, id4, id5);
        }

        // Read scanIDs.json for scan ids
        let scan_ids = json::parse(
            &std::fs::read_to_string("scanIDs.json").expect("Unable to read scanIDs.json"),
        )
        .expect("Failed to parse scanIDs.json");
        let epoch_preparation_scan_id = scan_ids["epoch_preparation_scan_id"].to_string();
        let live_epoch_scan_id = scan_ids["live_epoch_scan_id"].to_string();
        let datapoint_scan_id = scan_ids["datapoint_scan_id"].to_string();
        let pool_deposit_scan_id = scan_ids["pool_deposit_scan_id"].to_string();

        OraclePool {
            local_oracle_address: local_oracle_address,
            oracle_pool_nft: oracle_pool_nft,
            oracle_pool_participant_token: oracle_pool_participant_token,
            epoch_preparation_stage: Stage {
                contract_address: epoch_preparation_contract_address,
                scan_id: epoch_preparation_scan_id,
            },
            live_epoch_stage: Stage {
                contract_address: live_epoch_contract_address,
                scan_id: live_epoch_scan_id,
            },
            datapoint_stage: Stage {
                contract_address: datapoint_contract_address,
                scan_id: datapoint_scan_id,
            },
            pool_deposit_stage: Stage {
                contract_address: pool_deposit_contract_address,
                scan_id: pool_deposit_scan_id,
            },
        }
    }

    /// Get the current stage of the oracle pool box. Returns either `Preparation` or `Epoch`.
    pub fn check_oracle_pool_stage(&self) -> PoolBoxState {
        match self.get_live_epoch_state() {
            Some(_) => PoolBoxState::LiveEpoch,
            None => PoolBoxState::Preparation,
        }
    }

    /// Get the state of the current oracle pool epoch
    pub fn get_live_epoch_state(&self) -> Option<LiveEpochState> {
        let epoch_box = self.live_epoch_stage.get_box()?;
        let epoch_box_regs = epoch_box.additional_registers.get_ordered_values();
        let epoch_box_id: String = epoch_box.box_id().into();

        // Whether datapoint was commit in the current Live Epoch
        let datapoint_state = self.get_datapoint_state()?;
        let commit_datapoint_in_epoch: bool = epoch_box_id == datapoint_state.origin_epoch_id;

        // Latest pool datapoint is held in R4 of the epoch box
        let latest_pool_datapoint = deserialize_integer(&epoch_box_regs[0])?;

        // Block height epochs ends is held in R5 of the epoch box
        let epoch_ends = deserialize_integer(&epoch_box_regs[1])?;

        let epoch_state = LiveEpochState {
            funds: epoch_box.value.value(),
            epoch_id: epoch_box_id,
            commit_datapoint_in_epoch: commit_datapoint_in_epoch,
            epoch_ends: epoch_ends as u64,
            latest_pool_datapoint: latest_pool_datapoint as u64,
        };

        Some(epoch_state)
    }

    /// Get the state of the current epoch preparation box
    pub fn get_preparation_state(&self) -> Option<PreparationState> {
        let epoch_prep_box = self.epoch_preparation_stage.get_box()?;
        let epoch_prep_box_regs = epoch_prep_box.additional_registers.get_ordered_values();

        // Latest pool datapoint is held in R4
        let latest_pool_datapoint = deserialize_integer(&epoch_prep_box_regs[0])?;

        // Next epoch ends height held in R5
        let next_epoch_ends = deserialize_integer(&epoch_prep_box_regs[1])?;

        let prep_state = PreparationState {
            funds: epoch_prep_box.value.value(),
            next_epoch_ends: next_epoch_ends as u64,
            latest_pool_datapoint: latest_pool_datapoint as u64,
        };

        Some(prep_state)
    }

    /// Get the current state of the local oracle's datapoint
    pub fn get_datapoint_state(&self) -> Option<DatapointState> {
        let datapoint_box = self.datapoint_stage.get_box()?;
        let datapoint_box_regs = datapoint_box.additional_registers.get_ordered_values();

        // The Live Epoch box id of the epoch the datapoint was posted in (which is held in R5)
        let origin_epoch_id = deserialize_string(&datapoint_box_regs[1])?;

        // Oracle datapoint held in R6
        let datapoint = deserialize_integer(&datapoint_box_regs[2])?;

        let datapoint_state = DatapointState {
            datapoint: datapoint as u64,
            origin_epoch_id: origin_epoch_id,
        };

        Some(datapoint_state)
    }

    /// Get the current state of all of the pool deposit boxes
    pub fn get_pool_deposits_state(&self) -> Option<PoolDepositsState> {
        let deposits_box_list = self.pool_deposit_stage.get_boxes()?;

        // Sum up all Ergs held in pool deposit boxes
        let sum_ergs = deposits_box_list
            .iter()
            .fold(0, |acc, b| acc + b.value.value());

        let deposits_state = PoolDepositsState {
            number_of_boxes: deposits_box_list.len() as u64,
            total_nanoergs: sum_ergs,
        };

        Some(deposits_state)
    }
}

impl Stage {
    /// Returns all boxes held at the given stage based on the registered scan
    pub fn get_boxes(&self) -> Option<Vec<ErgoBox>> {
        get_scan_boxes(&self.scan_id)
    }

    /// Returns the first box found by the registered scan for a given `Stage`
    pub fn get_box(&self) -> Option<ErgoBox> {
        self.get_boxes()?.into_iter().nth(0)
    }

    /// Returns all boxes held at the given stage based on the registered scan
    /// serialized and ready to be used as rawInputs
    pub fn get_serialized_boxes(&self) -> Option<Vec<String>> {
        serialize_boxes(&self.get_boxes()?)
    }

    /// Returns the first box found by the registered scan for a given `Stage`
    /// serialized and ready to be used as a rawInput
    pub fn get_serialized_box(&self) -> Option<String> {
        serialize_box(&self.get_box()?)
    }
}
