/// This is a small library which wraps the `connector-lib` library for
/// Connectors which wish to plug into the Ergo Explorer Oracle Pool Frontend.
use connector_lib::Connector;

/// The data which the Oracle Pool Frontend uses
#[derive(Clone)]
pub struct FrontEndData {
    pub latest_price: u64,
    pub posting_schedule_minutes: u64,
    pub epoch_ends_in_minutes: u64,
    pub current_pool_stage: String,
    pub pool_funded_percentage: u64,
    pub number_of_oracles: u64,
    pub posting_schedule_blocks: u64,
    pub latest_datapoint: u64,
    pub live_epoch_address: String,
    pub epoch_prep_address: String,
    pub pool_deposits_address: String,
    pub datapoint_address: String,
    pub oracle_payout_price: u64,
    pub live_epoch_length: u64,
    pub epoch_prep_length: u64,
    pub outlier_range: u64,
    pub oracle_pool_nft_id: String,
    pub oracle_pool_participant_token_id: String,
    pub epoch_end_height: u64,
}

/// A `Connector` which is also built to support the Oracle Pool Frontend
#[derive(Clone)]
pub struct FrontEndConnector {
    connector: Connector,
    generate_frontend_data: fn(u64) -> FrontEndData,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
