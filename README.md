# Oracle Core v2.0

The oracle core requires that the user has access to a full node wallet in order to create txs & perform UTXO-set scanning. Furthermore each oracle core is designed to work with only a single oracle pool. If an operator runs several oracles in several oracle pools then a single full node can be used, but several instances of oracle cores must be run (and set with different api ports).

The current oracle core is built to run the protocol specified in the [EIP-0023 PR](https://github.com/ergoplatform/eips/pull/41).

## Roadmap

### In progress

- v2.0-beta. Run a public oracle pool on testnet. See [planned tasks](https://github.com/ergoplatform/oracle-core/milestone/5)

### Next

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

## Setup

Generate an oracle config file from the default template with:

```console
oracle-core generate-oracle-config
```

and set the required parameters:

- `oracle_address` - a node's address that will be used by this oracle-core instance(pay tx fees, keep tokens, etc.). Make sure it has coins;
- `node_ip`, `node_port`, `node_api_key` - node connection parameters;

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

to mint tokens and create pool, refresh, update boxes. The `pool_config.yaml` file will be generated. It contains the configuration needed to run this pool;

- Run an oracle with

``` console
oracle-core run
```

Bootstrap parameters available to edit:

- `[token]:name`, `description` - token names and descriptions that will be used to mint tokens;
- `[token]:quantity` - number of tokens to mint;
- `data_point_source` - can be one of the following: NanoErgUsd, NanoErgXau, NanoErgAda;
- `min_data_points` - minimal number of posted datapoint boxes needed to update the pool box (consensus);
- `max_deviation_percent` - a cut off for the lowest and highest posted datapoints(i.e. datapoints deviated more than this will be filtered out and not take part in the refresh of the pool box);
- `epoch_length` - minimal number of blocks between refresh(pool box) actions;
- `min_votes` - minimal number of posted ballot boxes voting for a change to the pool box contracts;
- `min_storage_rent` - box value in nanoERG used in oracle and ballot boxes;

Check out [How I bootstrapped an ERG/XAU pool on testnet](docs/how_to_bootstrap.md) report for an example.

## Invite new oracle to the running pool

To invite a new oracle the person that bootstrapped the pool need to send one oracle token and one reward token. On bootstrap X oracle and reward tokens are sent to the `oracle_address`, where X is the total oracle token quantity minted on bootstrap.
Use [scripts/send_new_oracle.sh](scripts/send_new_oracle.sh) to send one oracle, reward and ballot token.
Besides the tokens the pool config file that you are running now should be sent as well. Send `pool_config.yaml` to the new oracle.

## Joining a running pool

To join the existing pool one oracle and one reward token must be received to the address which will be used as `oracle_address` in the config file of the oracle. The received `pool_config.yaml` config file must placed accordingly.

To run the oracle:

- Make sure node's wallet is unlocked;
- Run an oracle with

``` console
oracle-core run
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

## Transfer the oracle token to a new operator

Be aware that reward tokens currently accumulated in the oracle box are transferred as well.
Run

``` console
oracle-core transfer-oracle-token <ADDRESS>
```

Ensure the new address has enough coins for tx fees to run in a pool.
As with inviting a new oracle, the pool config file that you are running now should be sent as well. Send `pool_config.yaml` to the new operator.

## Updating the contracts/tokens

Changes to the contract(parameters)/tokens can be done in three steps:

- `prepare-update` command submits a new refresh box with the updated refresh contract;
- `vote-update-pool` command submits oracle's ballot box voting for the changes;
- `update-pool` command submits the update transaction, which produces a new pool box;
Each of the step is described below. See also a detailed instruction on [Updating the epoch length](docs/update_epoch_length.md)

### Create a new refresh box with `prepare-update` command

Create a YAML file describing what contract parameters should be updated.
See also an example of such YAML file at [Updating the epoch length](docs/update_epoch_length.md)
Run:

```console
oracle-core prepare-update <YAML file>
```

This will generate `pool_config_updated.yaml` config file which should be used in `update-pool` command.
The output shows the new pool box contract hash and reward tokens amounts for the subsequent dozen epochs. To be used in the `vote-update-pool` command run by the oracles on the next step.

### Vote for contract update with `vote-update-pool` command

Run

```console
oracle-core vote-update-pool <NEW_POOL_BOX_ADDRESS_HASH_STR> <REWARD_TOKEN_ID_STR> <REWARD_TOKEN_AMOUNT> <UPDATE_BOX_CREATION_HEIGHT>
```

Where:

- <NEW_POOL_BOX_ADDRESS_HASH_STR> - base16-encoded blake2b hash of the serialized pool box contract for the new pool box
- <REWARD_TOKEN_ID_STR> - base16-encoded reward token id in the new pool box (use existing if unchanged)
- <REWARD_TOKEN_AMOUNT> - reward token amount in the pool box at the time of update transaction is committed
- <UPDATE_BOX_CREATION_HEIGHT> - The creation height of the existing update box.

and are printed in the output of the `prepare-update` command.

Keep in mind the REWARD_TOKEN_AMOUNT depends on when(in which epoch) the final `update-pool` command will be run.

### Update the pool box contract with `update-pool` command

Make sure the `pool_config_updated.yaml` config file generated during the `prepare-update` command is in the same folder as the oracle-core binary.
Run

```console
oracle-core update-pool 
```

to see the diff for the tokens.
Run

```console
oracle-core update-pool <NEW_POOL_BOX_ADDRESS_HASH_STR> <REWARD_TOKEN_ID_STR> <REWARD_TOKEN_AMOUNT> 
```

Where:
  <NEW_POOL_BOX_ADDRESS_HASH_STR> - base16-encoded blake2b hash of the serialized pool box contract for the new pool box
  <REWARD_TOKEN_ID_STR> - base16-encoded reward token id in the new pool box (use existing if unchanged)
  <REWARD_TOKEN_AMOUNT> - reward token amount in the pool box at the time of update transaction is committed

This will submit an update tx.
After the update tx is confirmed, remove `scanIds.json` and use `pool_config_updated.yaml` to run the oracle (i.e., rename it to `pool_config.yaml` and restart the oracle).
Distribute the `pool_config.yaml` file to all the oracles. Be sure they delete `scanIds.json` before restart.

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
