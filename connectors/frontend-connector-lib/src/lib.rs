/// This is a small library which wraps the `connector-lib` library for
/// Connectors which wish to plug into the Ergo Explorer Oracle Pool Frontend.
#[macro_use]
extern crate json;

mod api;

use anyhow::Result;
use api::start_get_api;
use connector_lib::{get_core_api_port, Connector, Datapoint, OracleCore};
use std::thread;

type Price = f64;

/// A `Connector` which is also built to support the Oracle Pool Frontend
#[derive(Clone)]
pub struct FrontendConnector {
    /// The underlying `Connector`
    connector: Connector,
    /// The library user-defined function which performs the logic of going
    /// from a `Datapoint` that is encoded as `u64` for on-chain use, to a
    /// `Price` which is the human-readable `f64` value.
    generate_current_price: fn(Datapoint) -> Price,
}

impl FrontendConnector {
    /// Create a new FrontendConnector
    pub fn new(
        title: &str,
        get_datapoint: fn() -> Result<u64>,
        print_info: fn(&Connector, &OracleCore) -> Result<bool>,
        generate_current_price: fn(Datapoint) -> Price,
    ) -> FrontendConnector {
        let connector = Connector::new(title, get_datapoint, print_info);
        let frontend_connector = FrontendConnector {
            connector: connector,
            generate_current_price: generate_current_price,
        };
        start_get_api(frontend_connector.clone());
        frontend_connector
    }

    /// Create a new FrontendConnector with basic predefined printing
    pub fn new_basic_connector(
        title: &str,
        get_datapoint: fn() -> Result<u64>,
        generate_current_price: fn(Datapoint) -> Price,
    ) -> FrontendConnector {
        let connector = Connector::new_basic_connector(title, get_datapoint);
        FrontendConnector {
            connector: connector,
            generate_current_price: generate_current_price,
        }
    }

    /// Run the `FrontendConnector` using a local Oracle Core + start the GET
    /// API Server
    pub fn run(&self) {
        let frontend_connector = self.clone();
        // Starts the FrontendConnector GET API Server
        thread::Builder::new()
            .name("Frontend Connector API Thread".to_string())
            .spawn(move || {
                start_get_api(frontend_connector);
            })
            .ok();
        self.connector.run()
    }

    /// Generates the json for the frontend data
    pub fn prepare_frontend_data_json(&self) -> Result<String> {
        let oc = OracleCore::new("0.0.0.0", &get_core_api_port()?);
        let pinfo = oc.pool_info()?;
        let pstatus = oc.pool_status()?;
        let block_height = oc.current_block_height()?;

        // Posting Schedule
        let posting_sched_blocks = pinfo.live_epoch_length + pinfo.epoch_prep_length;
        let posting_sched_minutes = posting_sched_blocks * 2;

        // How Long Until Epoch Ends
        let mut epoch_ends_in_minutes = 0;
        if pstatus.epoch_ends > block_height {
            epoch_ends_in_minutes = (pstatus.epoch_ends - block_height) * 2;
        }

        let data_json = object! {
            // title: title,

            latest_price: (self.generate_current_price)(pstatus.latest_datapoint),
            posting_schedule_minutes: posting_sched_minutes,
            epoch_ends_in_minutes: epoch_ends_in_minutes,
            current_pool_stage: pstatus.current_pool_stage,
            pool_funded_percentage: pstatus.funded_percentage,

            number_of_oracles: pinfo.number_of_oracles,
            posting_schedule_blocks: posting_sched_blocks,


            latest_datapoint: pstatus.latest_datapoint,
            live_epoch_address : pinfo.live_epoch_address,
            epoch_prep_address: pinfo.epoch_prep_address,
            pool_deposits_address: pinfo.pool_deposits_address,
            datapoint_address: pinfo.datapoint_address,
            oracle_payout_price: pinfo.oracle_payout_price,
            live_epoch_length: pinfo.live_epoch_length,
            epoch_prep_length: pinfo.epoch_prep_length,
            outlier_range: pinfo.outlier_range,
            oracle_pool_nft_id: pinfo.oracle_pool_nft_id,
            oracle_pool_participant_token_id: pinfo.oracle_pool_participant_token_id,
            epoch_end_height: pstatus.epoch_ends,
        };

        Ok(data_json.dump())
    }
}
