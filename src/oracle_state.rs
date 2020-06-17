/// This files relates to the state of the oracle/oracle pool.
use crate::node_interface::{register_scan, get_scan_boxes};
use crate::oracle_config::{get_config_yaml, get_node_url, get_node_api_key};
use crate::{NanoErg, BlockHeight, EpochID};
use crate::scans::{save_scan_ids_locally, register_epoch_preparation_scan, register_oracle_pool_epoch_scan, register_datapoint_scan, register_pool_deposit_scan};
use std::path::Path;
use yaml_rust::{YamlLoader};


/// Overarching Trait object for `PreparationState` and `EpochState`
pub trait OraclePoolState {
    fn stage(&self) -> PoolStage;
}


#[derive(Debug, Clone)]
/// Enum for the oracle pool box stage
pub enum PoolStage { 
    Preparation,
    Epoch
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
#[derive(Debug, Clone)]
pub struct OraclePool {
    /// Address of the local oracle running the oracle core
    pub local_oracle_address: String,
    /// Token IDs
    pub oracle_pool_nft: String,
    pub oracle_pool_participant_token: String,
    /// Contracts Addresses
    pub epoch_preparation_contract_address: String,
    pub oracle_pool_epoch_contract_address: String,
    pub datapoint_contract_address: String,
    pub pool_deposit_contract_address: String,
    /// Scan IDs
    pub epoch_preparation_scan_id: String,
    pub oracle_pool_epoch_scan_id: String,
    pub datapoint_scan_id: String,
    pub pool_deposit_scan_id: String,


}


impl OraclePool {

    /// Create a new `OraclePool` struct
    pub fn new() -> OraclePool {
        let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];

        let local_oracle_address = config["oracle_address"].as_str().expect("No oracle_pool_nft specified in config file.").to_string();
        let oracle_pool_nft = config["oracle_pool_nft"].as_str().expect("No oracle_pool_nft specified in config file.").to_string();
        let oracle_pool_participant_token = config["oracle_pool_participant_token"].as_str().expect("No oracle_pool_participant_token specified in config file.").to_string();
        
        let epoch_preparation_contract_address = config["epoch_preparation_contract_address"].as_str().expect("No epoch_preparation_contract_address specified in config file.").to_string();
        let oracle_pool_epoch_contract_address = config["oracle_pool_epoch_contract_address"].as_str().expect("No oracle_pool_epoch_contract_address specified in config file.").to_string();
        let datapoint_contract_address = config["datapoint_contract_address"].as_str().expect("No datapoint_contract_address specified in config file.").to_string();
        let pool_deposit_contract_address = config["pool_deposit_contract_address"].as_str().expect("No pool_deposit_contract_address specified in config file.").to_string();

        // If scanIDs.json exists, skip registering scans & saving generated ids
        if !Path::new("scanIDs.json").exists() {
            // Add registering here and calling save_scan_ids_locally with returned ids
            let id1 = register_epoch_preparation_scan(&oracle_pool_nft, &epoch_preparation_contract_address);
            let id2 = register_oracle_pool_epoch_scan(&oracle_pool_nft, &oracle_pool_epoch_contract_address);
            let id3 = register_datapoint_scan(&oracle_pool_participant_token, &datapoint_contract_address, &local_oracle_address);
            let id4 = register_pool_deposit_scan(&pool_deposit_contract_address);

            save_scan_ids_locally(id1, id2, id3, id4);
            // save_scan_ids_locally("1234".to_string(), "1234".to_string(),"1234".to_string(),"1234".to_string());
        }

        // Read scanIDs.json for scan ids
        let scan_ids = json::parse(&std::fs::read_to_string("scanIDs.json").expect("Unable to read scanIDs.json")).expect("Failed to parse scanIDs.json");
        let epoch_preparation_scan_id = scan_ids["epoch_preparation_scan_id"].to_string();
        let oracle_pool_epoch_scan_id = scan_ids["oracle_pool_epoch_scan_id"].to_string();
        let datapoint_scan_id = scan_ids["datapoint_scan_id"].to_string();
        let pool_deposit_scan_id = scan_ids["pool_deposit_scan_id"].to_string();


        OraclePool {
            local_oracle_address: local_oracle_address,
            oracle_pool_nft: oracle_pool_nft,
            oracle_pool_participant_token: oracle_pool_participant_token,
            epoch_preparation_contract_address: epoch_preparation_contract_address,
            oracle_pool_epoch_contract_address: oracle_pool_epoch_contract_address,
            datapoint_contract_address: datapoint_contract_address,
            pool_deposit_contract_address: pool_deposit_contract_address,
            epoch_preparation_scan_id: epoch_preparation_scan_id,
            oracle_pool_epoch_scan_id: oracle_pool_epoch_scan_id,
            datapoint_scan_id: datapoint_scan_id,
            pool_deposit_scan_id: pool_deposit_scan_id,
        }


    }

    // Get the current state of the oracle pool box. Returns trait object OraclePoolState which may be either `EpochState` or `PreparationState`.
    // pub fn get_oracle_pool_state (&self) -> dyn OraclePoolState {
    //     let epoch_preparation_box_list = get_scan_boxes(self.epoch_preparation_scan_id);
    //     let pool_epoch_box_list = get_scan_boxes(self.oracle_pool_epoch_scan_id);
    // }

    // Get the current state of the local oracle's datapoint
    // pub fn get_datapoint_state(&self) -> DatapointState {
    // }

    // Get the current state of all of the pool deposit boxes
    // pub fn get_pool_deposits_state(&self) -> PoolDepositsState {
    // }
}


/// The state of the oracle pool when it is in the Oracle Pool Epoch stage
#[derive(Debug, Clone)]
pub struct EpochState {
    pub funds: NanoErg,
    pub epoch_id: EpochID,
    pub commit_datapoint_in_epoch: bool,
    pub epoch_ends: BlockHeight,
}

/// The state of the oracle pool when it is in the Epoch Preparation stage
#[derive(Debug, Clone)]
pub struct PreparationState {
    pub funds: NanoErg,
    pub next_epoch_ends: BlockHeight,
}

/// The state of the local oracle's Datapoint box
#[derive(Debug, Clone)]
pub struct DatapointState {
    datapoint: String,
    from_epoch: EpochID,
}

/// The current UTXO-set state of all of the Pool Deposit boxes
#[derive(Debug, Clone)]
pub struct PoolDepositsState {
    number_of_boxes: u64,
    total_ergs: u64
}



impl OraclePoolState for EpochState {
    fn stage(&self) -> PoolStage {
        PoolStage::Epoch
    }
}


impl OraclePoolState for PreparationState {
    fn stage(&self) -> PoolStage {
        PoolStage::Preparation
    }
}



