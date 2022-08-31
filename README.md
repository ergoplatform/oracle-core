# Oracle Core v2.0

The oracle core is the off-chain component that oracles who are part of an oracle pool run. This oracle core provides a HTTP API interface for reading the current protocol state & another for submitting datapoints. Once a datapoint is submited, the oracle core will automatically generate the required tx and post it as well as any other actions required for the protocol to run. This thereby allows the oracle to participate in the oracle pool protocol without any extra effort for the oracle operator.

The oracle core requires that the user has access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools then a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).

The current oracle core is built to run the protocol specified in the [EIP-0023 PR](https://github.com/ergoplatform/eips/pull/41).

## Verifying contracts

It is recommended to check that the contracts used are indeed coming from EIP-23. Run the following command to get encoded hashes of each contract:
```console
cargo test print_contract_hashes -- --nocapture
```

Check these values against those described in EIP-23.


