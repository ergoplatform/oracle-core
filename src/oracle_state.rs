/// This files relates to the state of the oracle/oracle pool.
use yaml_rust::{YamlLoader};
use crate::node_interface::{register_scan};
use crate::oracle_config::{get_config_yaml, get_node_url, get_node_api_key};
use crate::{NanoErg, BlockHeight, EpochID};


/// Overarching Trait object for `PreparationState` and `EpochState`
pub trait OraclePoolBox {
    fn stage(&self) -> PoolStage;
}


#[derive(Debug, Clone)]
/// Enum for the oracle pool box stage
pub enum PoolStage { 
    Preparation,
    Epoch
}

/// Overarching struct which allows for acquiring the state of the whole oracle pool protocol
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

        // Add logic of opening of scanIDs.yaml and reading from them if available. Else register scans and create file/store scanIDs there automatically.
        let epoch_preparation_scan_id = "".to_string();
        let oracle_pool_epoch_scan_id = "".to_string();
        let datapoint_scan_id = "".to_string();
        let pool_deposit_scan_id = "".to_string();


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



impl OraclePoolBox for EpochState {
    fn stage(&self) -> PoolStage {
        PoolStage::Epoch
    }
}


impl OraclePoolBox for PreparationState {
    fn stage(&self) -> PoolStage {
        PoolStage::Preparation
    }
}



