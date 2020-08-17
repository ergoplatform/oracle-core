/// This is a small library which wraps the `connector-lib` library for
/// Connectors which wish to plug into the Ergo Explorer Oracle Pool Frontend.
mod api;

use anyhow::Result;
use api::start_get_api;
use connector_lib::{Connector, Datapoint, OracleCore};

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
}
