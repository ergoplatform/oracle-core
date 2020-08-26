Frontend Connector Lib
--------------------------

This is a small library which wraps the `connector-lib` library for Connectors which wish to plug into the Ergo Explorer Oracle Pool Frontend.

This library exposes your connector via an API server with a port that is `+2` the port of the local Oracle Core. It provides an endpoint called `/frontendData` which provides the Oracle Pool Frontend all of the required data it needs in order to visualize your oracle pool.


How To Use
-----------

To get started using this library you must create a `FrontEndConnector`. This is like your average `Connector` from `connector-lib`, however it also requires providing another function as input:

```rust
generate_current_price: fn(Datapoint) -> Price
```

This is a function which performs the logic of going from a `Datapoint` that is encoded as `u64` for on-chain use, to a `Price` which is the human-readable `f64` value. This is needed by the frontend to display to end-users who will much prefer to have `$0.43` rather than `2282891327` for example.


The following is an example of creating and using a `FrontEndConnector`:

```rust
...
```
