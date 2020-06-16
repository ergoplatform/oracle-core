/// This file holds logic related to UTXO-set scans


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