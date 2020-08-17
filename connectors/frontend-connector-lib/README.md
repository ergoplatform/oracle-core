Frontend Connector Lib
--------------------------

This is a small library which wraps the `connector-lib` library for Connectors which wish to plug into the Ergo Explorer Oracle Pool Frontend.

This library exposes your connector via an API server with a port that is `+2` the port of the local Oracle Core. It provides an endpoint called `/frontendData` which provides the Oracle Pool Frontend all of the required data it needs in order to visualize your oracle pool.


How To Use
-----------

To get started using this library you must create a `FrontEndConnector`. This is like your average `Connector` from `connector-lib`, however it also requires providing another function as input:

```rust
generate_frontend_data(datapoint: u64) -> FrontEndData
```

This is a function which the developer must write which creates a `FrontEndData` struct. This is a struct defined as:

```rust
Frontenddata definition here
```

Customizing this function provides the developer freedom in how their data is transformed before it is displayed in the Oracle Pool Frontend.

The following is an example of creating a `FrontEndConnector`:


```rust
...
```
