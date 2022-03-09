/// This files relates to the state of the oracle/oracle pool.
use crate::oracle_config::get_config_yaml;
use crate::scans::{
    register_datapoint_scan, register_epoch_preparation_scan, register_live_epoch_scan,
    register_local_oracle_datapoint_scan, register_pool_deposit_scan, save_scan_ids_locally, Scan,
};
use crate::Result;
use crate::{BlockHeight, EpochID, NanoErg, P2PKAddress, TokenID};
use ergo_lib::chain::ergo_box::ErgoBox;
use ergo_offchain_utilities::encoding::{unwrap_hex_encoded_string, unwrap_int, unwrap_long};
use std::path::Path;
use yaml_rust::YamlLoader;

/// Enum for the state that the oracle pool box is currently in
#[derive(Debug, Clone)]
pub enum PoolBoxState {
    Preparation,
    LiveEpoch,
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
    /// Token IDs
    pub oracle_pool_nft: TokenID,
    pub oracle_pool_participant_token: TokenID,
    /// Stages
    pub epoch_preparation_stage: Stage,
    pub live_epoch_stage: Stage,
    pub datapoint_stage: Stage,
    pub pool_deposit_stage: Stage,
    // Local Oracle Datapoint Scan
    pub local_oracle_datapoint_scan: Scan,
}

/// The state of the oracle pool when it is in the Live Epoch stage
#[derive(Debug, Clone)]
pub struct LiveEpochState {
    pub funds: NanoErg,
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
    /// Box id of the epoch which the datapoint was posted in/originates from
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
            let scans = vec![
                register_epoch_preparation_scan(
                    &oracle_pool_nft,
                    &epoch_preparation_contract_address,
                )
                .unwrap(),
                register_live_epoch_scan(&oracle_pool_nft, &live_epoch_contract_address).unwrap(),
                register_local_oracle_datapoint_scan(
                    &oracle_pool_participant_token,
                    &datapoint_contract_address,
                    &local_oracle_address,
                )
                .unwrap(),
                register_datapoint_scan(
                    &oracle_pool_participant_token,
                    &datapoint_contract_address,
                )
                .unwrap(),
                register_pool_deposit_scan(&pool_deposit_contract_address).unwrap(),
            ];
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
            &"Epoch Preparation Scan".to_string(),
            &scan_json["Epoch Preparation Scan"].to_string(),
        );
        let live_epoch_scan = Scan::new(
            &"Live Epoch Scan".to_string(),
            &scan_json["Live Epoch Scan"].to_string(),
        );
        let datapoint_scan = Scan::new(
            &"All Oracle Datapoints Scan".to_string(),
            &scan_json["All Datapoints Scan"].to_string(),
        );
        let local_oracle_datapoint_scan = Scan::new(
            &"Local Oracle Datapoint Scan".to_string(),
            &scan_json["Local Oracle Datapoint Scan"].to_string(),
        );
        let pool_deposit_scan = Scan::new(
            &"Pool Deposits Scan".to_string(),
            &scan_json["Pool Deposits Scan"].to_string(),
        );

        // Create `OraclePool` struct
        OraclePool {
            local_oracle_address,
            oracle_pool_nft,
            oracle_pool_participant_token,
            epoch_preparation_stage: Stage {
                contract_address: epoch_preparation_contract_address,
                scan: epoch_preparation_scan,
            },
            live_epoch_stage: Stage {
                contract_address: live_epoch_contract_address,
                scan: live_epoch_scan,
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
        }
    }

    /// Get the current stage of the oracle pool box. Returns either `Preparation` or `Epoch`.
    pub fn check_oracle_pool_stage(&self) -> PoolBoxState {
        match self.get_live_epoch_state() {
            Ok(_) => PoolBoxState::LiveEpoch,
            Err(_) => PoolBoxState::Preparation,
        }
    }

    /// Get the state of the current oracle pool epoch
    pub fn get_live_epoch_state(&self) -> Result<LiveEpochState> {
        let epoch_box = self.live_epoch_stage.get_box()?;
        let epoch_box_regs = epoch_box.additional_registers.get_ordered_values();
        let epoch_box_id: String = epoch_box.box_id().into();

        // Whether datapoint was commit in the current Live Epoch
        let datapoint_state = self.get_datapoint_state()?;
        let commit_datapoint_in_epoch: bool = epoch_box_id == datapoint_state.origin_epoch_id;

        // Latest pool datapoint is held in R4 of the epoch box
        let latest_pool_datapoint = unwrap_long(&epoch_box_regs[0])?;

        // Block height epochs ends is held in R5 of the epoch box
        let epoch_ends = unwrap_int(&epoch_box_regs[1])?;

        let epoch_state = LiveEpochState {
            funds: *epoch_box.value.as_u64(),
            epoch_id: epoch_box_id,
            commit_datapoint_in_epoch,
            epoch_ends: epoch_ends as u64,
            latest_pool_datapoint: latest_pool_datapoint as u64,
        };

        Ok(epoch_state)
    }

    /// Get the state of the current epoch preparation box
    pub fn get_preparation_state(&self) -> Result<PreparationState> {
        let epoch_prep_box = self.epoch_preparation_stage.get_box()?;
        let epoch_prep_box_regs = epoch_prep_box.additional_registers.get_ordered_values();

        // Latest pool datapoint is held in R4
        let latest_pool_datapoint = unwrap_long(&epoch_prep_box_regs[0])?;

        // Next epoch ends height held in R5
        let next_epoch_ends = unwrap_int(&epoch_prep_box_regs[1])?;

        let prep_state = PreparationState {
            funds: *epoch_prep_box.value.as_u64(),
            next_epoch_ends: next_epoch_ends as u64,
            latest_pool_datapoint: latest_pool_datapoint as u64,
        };

        Ok(prep_state)
    }

    /// Get the current state of the local oracle's datapoint
    pub fn get_datapoint_state(&self) -> Result<DatapointState> {
        let datapoint_box = self.local_oracle_datapoint_scan.get_box()?;
        let datapoint_box_regs = datapoint_box.additional_registers.get_ordered_values();

        // The Live Epoch box id of the epoch the datapoint was posted in (which is held in R5)
        let origin_epoch_id = unwrap_hex_encoded_string(&datapoint_box_regs[1])?;

        // Oracle datapoint held in R6
        let datapoint = unwrap_long(&datapoint_box_regs[2])?;

        let datapoint_state = DatapointState {
            datapoint: datapoint as u64,
            origin_epoch_id: origin_epoch_id.clone(),
            creation_height: datapoint_box.creation_height as u64,
        };

        Ok(datapoint_state)
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
}

impl Stage {
    /// Returns all boxes held at the given stage based on the registered scan
    pub fn get_boxes(&self) -> Result<Vec<ErgoBox>> {
        self.scan.get_boxes()
    }

    /// Returns the first box found by the registered scan for a given `Stage`
    pub fn get_box(&self) -> Result<ErgoBox> {
        self.scan.get_box()
    }

    /// Returns all boxes held at the given stage based on the registered scan
    /// serialized and ready to be used as rawInputs
    pub fn get_serialized_boxes(&self) -> Result<Vec<String>> {
        self.scan.get_serialized_boxes()
    }

    /// Returns the first box found by the registered scan for a given `Stage`
    /// serialized and ready to be used as a rawInput
    pub fn get_serialized_box(&self) -> Result<String> {
        self.scan.get_serialized_box()
    }

    /// Returns the number of boxes held at the given stage based on the registered scan
    pub fn number_of_boxes(&self) -> Result<u64> {
        Ok(self.get_boxes()?.len() as u64)
    }
}
