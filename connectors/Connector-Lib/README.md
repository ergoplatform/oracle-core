# Connector Lib

This is a small crate which provides essential functions required for interfacing with the oracle core.

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