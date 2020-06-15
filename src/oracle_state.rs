/// This files relates to the state of the oracle/oracle pool.
/// It provides the functions for setting up the required scans to follow the given oracle pool
/// as well as checking the scans in regular intervals and generating structs from the results.
use crate::node_interface::{register_scan};
use crate::oracle_config::{get_oracle_pool_nft_id};


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