# Connector Lib

This is a small framework for creating `Connector`s for an Oracle Pool. These connectors act as a middleman which interface between the outside world and the Oracle Core.


Building A Basic Connector
==========================
In short, when building a basic connector you only need to define three things:
1. The title of the Connector (which explains what the datapoint is).
2. A description of the Connector (which explains in greater detail how the data is sourced/processed)
3. A function which fetches & processes the datapoint from an external service and returns it as a Result<u64>.

Basic example for fetching nanoErg per 1 USD:

```rust
use anyhow::{anyhow, Result};
use connector_lib::Connector;
use json;

/// Acquires the nanoErg/USD price from CoinGecko
fn get_nanoerg_usd_price() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["ergo"]["usd"].as_f64() {
        let nanoerg_price = (1.0 / p as f64) * 1000000000.0;
        return Ok(nanoerg_price as u64);
    } else {
        Err(anyhow!("Failed to parse price."))
    }
}

fn main() {
    let connector = Connector::new_basic_connector(
        "ERG-USD",
        "Connector which fetches the number of nanoErgs per 1 USD.",
        get_nanoerg_usd_price,
    );
    connector.run();
}
```




Building Custom Connectors
==========================






















Advanced Usage
=================
You can also directly interact with the locally running Oracle Core if you have an advanced use case for your Connector.

In short, simply create a `OracleCore` struct and use the methods on it in order to acquire the state of the Oracle/Pool, block height, and to submit datapoints.
```rust
    let oc = OracleCore::new("0.0.0.0", "9090");`
    oc.oracle_info();
    oc.pool_info();
    oc.node_info();
    oc.oracle_status();
    oc.pool_status();
    oc.current_block_height();
    oc.submit_datapoint(259900000);
```