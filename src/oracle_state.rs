/// This files relates to the state of the oracle/oracle pool.
/// It provides the functions for setting up the required scans to follow the given oracle pool
/// as well as checking the scans in regular intervals and generating structs from the results.
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
    pub epoch_preparation_scan_id: Option<String>,
    pub oracle_pool_epoch_scan_id: Option<String>,
    pub datapoint_scan_id: Option<String>,
    pub pool_deposit_scan_id: Option<String>,


}


impl OraclePool {
    // Create methods to acquire said values
    // pub current_block_height: BlockHeight,
    // pub datapoint_state: DatapointState,
    // pub deposits_state: PoolDepositsState,
    // pub pool_box_state: dyn OraclePoolBox,
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




/// This function registers scanning for the Epoch Preparation stage box
pub fn register_epoch_preparation_scan() -> Option<String> {
    // Scan for NFT id + Epoch Preparation address
    None
}

/// This function registers scanning for the Oracle Pool Epoch stage box
pub fn register_oracle_pool_epoch_scan() -> Option<String> {
    // Scan for NFT id + Oracle Pool Epoch address
    None
}

/// This function registers scanning for the oracle's personal Datapoint box
pub fn register_datapoint_scan() -> Option<String> {
    // Scan for pool participant token id + oracle-address in R4
    None
}

/// This function registers scanning for any boxes in the Pool Deposit stage address
pub fn register_pool_deposit_scan() -> Option<String> {
    // Scan for pool participant token id + oracle-address in R4
    None
}