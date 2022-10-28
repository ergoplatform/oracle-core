# Oracle Core v2.0

The oracle core is the off-chain component that oracles who are part of an oracle pool run. This oracle core provides a HTTP API interface for reading the current protocol state & another for submitting datapoints. Once a datapoint is submited, the oracle core will automatically generate the required tx and post it as well as any other actions required for the protocol to run. This thereby allows the oracle to participate in the oracle pool protocol without any extra effort for the oracle operator.

The oracle core requires that the user has access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools then a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).

The current oracle core is built to run the protocol specified in the [EIP-0023 PR](https://github.com/ergoplatform/eips/pull/41).

## Roadmap
### In progress
- v2.0-alpha. The first run of the oracle pool, testing how all the components work together. See [the remaining tasks](https://github.com/ergoplatform/oracle-core/milestone/1)
### Next
- v2.0-beta. Run a public oracle pool on testnet. See [planned tasks](https://github.com/ergoplatform/oracle-core/milestone/5)
- v2.0-RC. Launch on the mainnet. See [planned tasks](https://github.com/ergoplatform/oracle-core/milestone/4)


## Getting started
### Download
Get the latest release binary from [Releases](https://github.com/ergoplatform/oracle-core/releases)
Or install it from the source code with:
``` console
cargo install --path core
```
If you want to run it as systemd daemon check out [this](https://github.com/ergoplatform/oracle-core#how-to-run-as-systemd-daemon) section.
Run it with `oracle-core --help` or `oracle-core <SUBCOMMAND> --help` to see the available commands and their options.

## Bootstrapping a new oracle pool
To bootstrap a new oracle pool:
- Run
``` console
oracle-core bootstrap --generate-config-template bootstrap.yaml
```
to generate an example of the bootstrap config file.
- Edit `bootstrap.yaml` (see the parameters list below);
- Make sure node's wallet is unlocked;
- Run 
``` console
oracle-core bootstrap bootstrap.yaml
``` 
to mint tokens and create pool, refresh, update boxes. The `oracle_config.yaml` file will be generated. It contains the configuration needed to run this pool;
- Run an oracle with 
``` console
oracle-core -c oracle_config.yaml run
```

Bootstrap parameters available to edit:
- `oracle_address` - a node's address that will be used by this oracle-core instance(pay tx fees, keep tokens, etc.). Make sure it has coins;
- `node_ip`, `node_port`, `node_api_key` - node connection parameters;
- `[token]:name`, `description` - token names and descriptions that will be used to mint tokens;
- `[token]:quantity` - number of tokens to mint;
- `data_point_source` - can be one of the following: NanoErgUsd, NanoErgXau, NanoErgAda;
- `data_point_source_custom_script` - path to script that will be called to fetch a new datapoint;
- `min_data_points` - minimal number of posted datapoint boxes needed to update the pool box (consensus);
- `max_deviation_percent` - a cut off for the lowest and highest posted datapoints(i.e. datapoints deviated more than this will be filtered out and not take part in the refresh of the pool box);
- `epoch_length` - minimal number of blocks between refresh(pool box) actions;
- `min_votes` - minimal number of posted ballot boxes voting for a change to the refresh/pool box contracts;
- `min_storage_rent` - box value in nanoERG used in oracle and ballot boxes;
- `base_fee` - a tx fee in nanoERG to use in transactions;

## Invite new oracle to the running pool
To invite a new oracle the person that bootstrapped the pool need to send one oracle token and one reward token. On bootstrap X oracle and reward tokens are sent to the `oracle_address`, where X is the total oracle token quantity minted on bootstrap.
Besides the tokens the `oracle_config.yaml` config file that you are running now should be sent as well. Be carefull to cleanup the `node_api_key` and `oracle_address` fields before you send it and instruct the invited oracle to set them to their liking.

## Joining a running pool
To join the existing pool one oracle and one reward token must be received to the address which will be used as `oracle_address` in the config file of the oracle. The received `oracle_config.yaml` config file must have the following fields updated to your setup:
- `oracle_address`;
- `node_api_key`;
- `node_ip`, `node_port` are set appropriately for your node;

To run the oracle:
- Make sure node's wallet is unlocked;
- Run an oracle with 
``` console
oracle-core -c oracle_config.yaml run
```

## Extract reward tokens
Since the earned reward tokens are accumulating in the oracle box there is a command to send all accumulated reward tokensminus 1 (needed for the contract) to the specified address:
``` console
oracle-core extract-reward-tokens <ADDRESS>
```
To show the amount of accumulated reward tokens in the oracle box run 
``` console
oracle-core print-reward-tokens
```

## How to run as systemd daemon
To run oracle-core as a systemd unit, the unit file in [systemd/oracle-core.service](systemd/oracle-core.service) should be installed.
The default configuration file path is ~/.config/oracle-core/oracle_config.yaml. This can be changed inside the .service file

``` console
cp systemd/oracle-core.service ~/.config/systemd/user/oracle-core.service
systemctl --user enable oracle-core.service
```

## Verifying contracts against EIP-23

It is recommended to check that the contracts used are indeed coming from EIP-23. Run the following command to get encoded hashes of each contract:
```console
./oracle-core print-contract-hashes
```
or if running from source files:
```console
cargo test check_contract_hashes -- --nocapture
```

Check these values against those described in EIP-23.
