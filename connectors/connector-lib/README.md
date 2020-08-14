# Connector Lib

This is a small framework for creating `Connector`s for an Oracle Pool. These connectors act as a middleware which interface between the outside world and the Oracle Core.


Building A Basic Connector
==========================
In short, when building a basic Connector you only need to define three things:
1. The title of the Connector (which explains what the datapoint is).
2. A description of the Connector (which explains in greater detail how the data is sourced/processed)
3. A function which fetches & processes the datapoint from an external service and returns it as a Result<u64>.

Basic example for creating a Erg-USD Connector which submits a nanoErg per 1 USD datapoint to the Oracle Core:

```rust
use anyhow::{anyhow, Result};
use connector_lib::Connector;
use json;

/// Acquires the price of Ergs in USD from CoinGecko and convert it
/// into nanoErgs per 1 USD.
fn get_datapoint() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["ergo"]["usd"].as_f64() {
        let nanoerg_price = (1.0 / p as f64) * 1000000000.0;
        return Ok(nanoerg_price as u64);
    } else {
        Err(anyhow!("Failed to parse price from json."))
    }
}

fn main() {
    let connector = Connector::new_basic_connector(
        "ERG-USD",
        "Connector which fetches the number of nanoErgs per 1 USD.",
        get_datapoint,
    );
    connector.run();
}
```

This is all the code that is required for setting up a basic Connector. Simply define a `get_datapoint` function, use `Connector::new_basic_connector` to create a new Connector instance, and then `connector.run()` to start the Connector.

The Connector runs a main loop which checks the state of the protocol from the Oracle Core every 30 seconds, and acquires/posts a datapoint if the protocol is in a valid state to accept it (meaning in `Live Epoch` stage and no datapoint accepted yet in current epoch).

The `Connector` struct also provides a `--bootstrap-value` flag automatically for you to use while bootstrapping your Oracle Pool.


Building Connectors Manually
==========================

If you wish to have more control over what the Connector prints then you can build a Connector manually.

You must define:
1. The title of the Connector (which explains what the datapoint is).
2. A description of the Connector (which explains in greater detail how the data is sourced/processed)
3. A function which fetches & processes the datapoint from an external service and returns it as a Result<u64>.
4. A function which prints information you deem important for your given connector.

Once you have defined all of the above data, simply call the `new()` method:

```rust
    let connector = Connector::new(
        title,
        description,
        get_datapoint,
        my_print_info_function,
    );
    connector.run();
```

This will then initiate and run the main loop of the Connector, and everything will run automatically for you.

Advanced Usage
=================
You also have the ability to directly interact with the locally running Oracle Core if you have an advanced use case.

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

This can be useful if for example you are expanding your Connector to include a publicly exposed API server.