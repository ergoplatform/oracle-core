# Oracle Core v2.0

Oracle Core is the off-chain component of an [EIP-0023](https://github.com/ergoplatform/eips/pull/41) oracle pool. Each instance posts datapoints for one pool and takes part in refreshing that pool's box. To serve several pools, run several instances against the same node, each with its own `core_api_port`, `metrics_port` and `--data-dir` (the log file `oracle-core.log` is written there).

## Contents

1. [Requirements](#1-requirements)
2. [Getting started](#2-getting-started)
    - 2.1 [Docker image](#21-docker-image)
    - 2.2 [Download or build](#22-download-or-build)
3. [Setup](#3-setup)
    - 3.1 [Keeping the mnemonic out of the shell history](#31-keeping-the-mnemonic-out-of-the-shell-history)
4. [Running](#4-running)
    - 4.1 [What to expect in the logs](#41-what-to-expect-in-the-logs)
5. [Price sources](#5-price-sources)
6. [Bootstrapping a new oracle pool](#6-bootstrapping-a-new-oracle-pool)
7. [Inviting a new oracle to a running pool](#7-inviting-a-new-oracle-to-a-running-pool)
8. [Joining a running pool](#8-joining-a-running-pool)
9. [Extracting reward tokens](#9-extracting-reward-tokens)
10. [Transferring the oracle token to a new operator](#10-transferring-the-oracle-token-to-a-new-operator)
11. [Updating the contracts and tokens](#11-updating-the-contracts-and-tokens)
    - 11.1 [Create a new refresh box with `prepare-update`](#111-create-a-new-refresh-box-with-prepare-update)
    - 11.2 [Vote for the update with `vote-update-pool`](#112-vote-for-the-update-with-vote-update-pool)
    - 11.3 [Update the pool box with `update-pool`](#113-update-the-pool-box-with-update-pool)
    - 11.4 [Import the updated pool config with `import-pool-update`](#114-import-the-updated-pool-config-with-import-pool-update)
12. [Running as a systemd daemon](#12-running-as-a-systemd-daemon)
13. [Verifying the contracts against EIP-23](#13-verifying-the-contracts-against-eip-23)
14. [Metrics](#14-metrics)
15. [Ubuntu install instructions](#15-ubuntu-install-instructions)
    - 15.1 [Install prerequisites](#151-install-prerequisites)
    - 15.2 [Clone and build Oracle Core](#152-clone-and-build-oracle-core)
    - 15.3 [Set up Oracle Core](#153-set-up-oracle-core)

---

## 1. Requirements

- An Ergo node you can reach over HTTP, with the extra indexer enabled (`ergo.node.extraIndex = true` in the node config). The oracle exits at startup if the indexer is not active, and waits if it is still catching up.
- No node API key and no node wallet. The oracle signs its own transactions with the secret you give it (see [Setup](#3-setup)) and only uses the node's public endpoints.
- An address that holds one oracle token, one reward token, one ballot token (only needed to vote on contract updates, section 11) and enough ERG for transaction fees. That address is derived from the secret; it is not configured separately.

---

## 2. Getting started

Two guided paths, pick one:

- **Docker**: pull the image and run it with your config folder mounted. See [Docker image](#21-docker-image).
- **Ubuntu, no Docker**: build from source and run the binary directly. See the step-by-step [Ubuntu install instructions](#15-ubuntu-install-instructions).

Everything else in this README is reference:

- the config keys, in [Setup](#3-setup)
- what the [price sources](#5-price-sources) are
- [what to expect in the logs](#41-what-to-expect-in-the-logs)
- the pool operations: [bootstrap](#6-bootstrapping-a-new-oracle-pool), [invite](#7-inviting-a-new-oracle-to-a-running-pool), [join](#8-joining-a-running-pool), [rewards](#9-extracting-reward-tokens), [contract updates](#11-updating-the-contracts-and-tokens)

### 2.1 Docker image

AMD64 and ARM64 images are available from the [Docker Hub repo](https://hub.docker.com/r/ergoplatform/oracle-core).

The container runs as user `oracle-core` (uid 9010). If you bind-mount the container's `/data` folder, where the config files and logs live, give the host folder to that uid (for example `chown -R 9010:9010 oracle_data`).

An example run command, after `source .secrets` (see [3.1](#31-keeping-the-mnemonic-out-of-the-shell-history)) so the mnemonic is in your environment:

```console
docker run -d \
 -v /path/on/host:/data \
 -p 9010:9010 \
 -p 9011:9011 \
 -e ORACLE_WALLET_MNEMONIC \
 ergoplatform/oracle-core:latest
```

The image starts the oracle with `--oracle-config-file /data/oracle_config.yaml --pool-config-file /data/pool_config.yaml -d /data run --enable-rest-api`. Port 9010 is the REST API and 9011 is for metrics if you enable them.

`-e ORACLE_WALLET_MNEMONIC` with no value copies the variable from your shell, so the mnemonic never appears on the command line or in your shell history. Note that `docker inspect` shows a container's environment to anyone with Docker access.

To enter the container shell for debugging or pool changes:

```console
docker exec -it -u oracle-core <container id> /bin/sh
```

### 2.2 Download or build

Get a release binary from [Releases](https://github.com/ergoplatform/oracle-core/releases), or build from source:

```console
cargo install --path core
```

Releases are built from tagged versions. Changes merged to `develop` after the latest tag, such as the current price-source set, are only in a build from source until the next release. The Docker image is built on the same releases, so its `latest` tag has the same lag.

Run `oracle-core --help` or `oracle-core <SUBCOMMAND> --help` to see the available commands and their options. Options that apply to every command:

- `--oracle-config-file <PATH>`: oracle config to use. Default `./oracle_config.yaml`.
- `--pool-config-file <PATH>`: pool config to use. Default `./pool_config.yaml`.
- `-d, --data-dir <PATH>`: folder for data files such as logs. Default is the current folder.
- `-v, --verbose`: more logging.

See [Ubuntu install instructions](#15-ubuntu-install-instructions) for a step-by-step build on a fresh machine.

---

## 3. Setup

Generate an oracle config file from the default template:

```console
oracle-core generate-oracle-config
```

This writes `oracle_config.yaml` in the current folder and prints a hint about "node credentials" and `oracle_address`; ignore it, neither is used. Set these parameters:

- `node_url`: URL of your Ergo node, for example `http://127.0.0.1:9053/`.
- `oracle_network`: `mainnet` or `testnet`.
- `oracle_secret` or `oracle_mnemonic`: the key the oracle signs with. Leave both as `null` and set the environment variable `ORACLE_WALLET_MNEMONIC` instead so the secret is never written to the config file (see [Keeping the mnemonic out of the shell history](#31-keeping-the-mnemonic-out-of-the-shell-history)). The oracle address is derived from this key at startup and logged as `Oracle Address derived from secret: 9...`. That address must hold the oracle token, the reward token and ERG for fees. It is the first address of the wallet (EIP-3 path `m/44'/429'/0'/0/0`, no passphrase); tokens on any other address of the same wallet are not seen. If you have the raw secret instead, the environment variable `ORACLE_WALLET_SECRET` is also accepted. Precedence: `ORACLE_WALLET_SECRET`, then `oracle_secret` in the file, then `ORACLE_WALLET_MNEMONIC`, then `oracle_mnemonic` in the file.
- `change_address`: optional. Where change from the oracle's transactions goes. Defaults to the oracle address.
- `base_fee`: transaction fee in nanoERG.
- `log_level`: `INFO` by default. `DEBUG` shows the per-iteration state (see [What to expect in the logs](#41-what-to-expect-in-the-logs)).
- `core_api_port`: port for the REST API, default 9010. Only used with `run --enable-rest-api`.
- `explorer_url`: explorer API base used by the CLI commands that wait for confirmations (bootstrap, prepare-update, extract and transfer). The same value is also used as the base of the `Check status:` links in the logs, so with the template's value `https://api.ergoplatform.com/` those links give 404 (replace `api.` with `explorer.` in the link). Set it to `null` to get a working link and the default API, or leave it and live with the dead link.
- `metrics_port`: set a port to enable Prometheus metrics (see [Metrics](#14-metrics)). `null` disables them.
- `data_point_source_custom_script`: optional path to a program that produces the datapoint instead of the built-in sources (see [Price sources](#5-price-sources)).
- `min_price_sources`: how many price sources must answer before a datapoint is posted. `erg_usd` (default 3) applies to ERG/USD and to the ERG leg of ERG/XAU. `gold_usd` (default 2) applies to the gold leg of ERG/XAU. It does not apply to `NanoAdaUsd` or `NanoErgBTC`. Leave it out or set it to `~` for the defaults.
- `livecoinwatch_api_key`: optional. A free key from livecoinwatch.com adds LiveCoinWatch as an ERG/USD source. Without it the other sources are used.

Keys that are not in the template are ignored. In particular `oracle_address` and `node_api_key` from older versions of this README have no effect.

Besides the oracle config, the oracle needs the pool's `pool_config.yaml` in the same folder (or the path given with `--pool-config-file`). You get it from the pool operator when [joining a running pool](#8-joining-a-running-pool), or from the [bootstrap](#6-bootstrapping-a-new-oracle-pool) if you create the pool.

### 3.1 Keeping the mnemonic out of the shell history

Put the mnemonic in a file that is only read by your shell:

```console
nano .secrets
```

with this content:

```bash
export ORACLE_WALLET_MNEMONIC="your twelve or fifteen or twenty-four word mnemonic phrase"
```

Make it readable only by you with `chmod 600 .secrets`. Load it before starting the oracle, in every new terminal and after every reboot:

```console
source .secrets
```

---

## 4. Running

```console
oracle-core run
```

Flags:

- `--enable-rest-api`: serve the status API on `core_api_port`, on all interfaces. Plain HTTP, no TLS.
- `--read-only`: builds the transactions (price sources are still queried) but never signs or submits them. At `INFO` the only trace is the refresh line `Refresh: Found N valid oracle boxes, next pool rate is ...`; a datapoint publish logs nothing. Run with `-v` or `log_level: DEBUG` to see every action it would take.

### 4.1 What to expect in the logs

At startup the oracle logs its version, the config files it loads and the derived oracle address. Check that address against the one holding your oracle token. If the node's indexer is behind, the oracle waits for it before doing anything else.

After that the oracle is quiet at the default `INFO` level until it acts. It logs a line for every datapoint it posts and every refresh it submits, with a link to the transaction, and logs errors. Silence is normal while it waits for its turn: after its datapoint box has been collected by a refresh, the oracle only re-posts in the second half of each epoch-length window counted from the current pool box, that is when `(height - pool box height) mod epoch_length > epoch_length / 2`. If the pool box is overdue for a refresh, that window comes round every epoch. With `log_level: DEBUG` every iteration logs the height and the live epoch state.

The REST API (`run --enable-rest-api`) serves `/oracleInfo`, `/oracleStatus`, `/poolInfo`, `/poolStatus`, `/oracleHealth` and `/poolHealth` as JSON, and `/blockHeight` as plain text, on `core_api_port` on all interfaces.

---

## 5. Price sources

`data_point_source` in `pool_config.yaml` selects what the pool prices. The predefined sources and where they fetch from:

- `NanoErgUsd`: nanoERG per 1 USD. Median of CoinPaprika, CoinGecko, Bitpanda, KuCoin, MEXC, Gate and CoinMarketCap, plus LiveCoinWatch when `livecoinwatch_api_key` is set. At least `min_price_sources.erg_usd` of them must answer.
- `NanoErgXau`: nanoERG per 1 kg of gold. Built from the ERG/USD median above and a gold/USD median of Bitpanda, COMEX and PAXG (Kraken and KuCoin, averaged into one vote). At least `min_price_sources.gold_usd` gold sources must answer. CoinGecko's direct ERG/XAU rate is averaged in when it answers, but is never used alone.
- `NanoAdaUsd`: CoinGecko only.
- `NanoErgBTC`: median of CoinGecko, CoinPaprika and Bitpanda.

Sources that fail or return stale data are skipped and logged by name at `WARN`. A fetched rate is reused for up to two minutes while the oracle retries a publish, so sources are not hit on every attempt.

`data_point_source_custom_script` in `oracle_config.yaml` replaces the predefined source with your own program. The oracle runs it with no arguments and parses its entire standard output as one integer: print only the digits, with no trailing newline or other text (for example `printf '%d' "$rate"`, not `echo`). The value is in the units the predefined source would post (for `NanoErgUsd`, nanoERG per 1 USD). The two-minute rate cache does not apply to a custom script; it is run on every attempt.

---

## 6. Bootstrapping a new oracle pool

Set up `oracle_config.yaml` and the mnemonic first ([Setup](#3-setup)): like every command, the bootstrap commands need them and a reachable node with the indexer active.

To bootstrap a new oracle pool:

- Generate an example bootstrap config:

```console
oracle-core bootstrap --generate-config-template bootstrap.yaml
```

- Edit `bootstrap.yaml` (see the parameters below). The oracle address must hold enough ERG to mint the tokens and create the pool, refresh and update boxes.
- Run

```console
oracle-core bootstrap bootstrap.yaml
```

to mint the tokens and create the pool, refresh and update boxes. This generates `pool_config.yaml` in the current folder (regardless of `--pool-config-file`) with the configuration needed to run the pool.

- Run an oracle with `oracle-core run`.

Bootstrap parameters:

- `[token]:name`, `description`: names and descriptions of the tokens to mint.
- `[token]:quantity`: number of tokens to mint.
- `data_point_source`: one of `NanoErgUsd`, `NanoErgXau`, `NanoAdaUsd`, `NanoErgBTC` (see [Price sources](#5-price-sources)).
- `min_data_points`: minimal number of posted datapoint boxes needed to update the pool box (consensus).
- `max_deviation_percent`: cut-off for the lowest and highest posted datapoints. Datapoints that deviate more than this are filtered out and do not take part in the refresh.
- `epoch_length`: minimal number of blocks between refreshes of the pool box.
- `min_votes`: minimal number of posted ballot boxes voting for a change to the pool box contracts.
- `min_storage_rent`: box value in nanoERG used in oracle and ballot boxes.
- `buffer_length`: the refresh contract's `buffer` from EIP-23, default 4: how many blocks the creation height of the pool box produced by a refresh may lag the current height. Leave the default unless you know why.

See [How I bootstrapped an ERG/XAU pool on testnet](docs/how_to_bootstrap.md) for a worked example. Its config section predates the current config format (`oracle_address`, node wallet); follow [Setup](#3-setup) for that part.

---

## 7. Inviting a new oracle to a running pool

To invite a new oracle, the person who bootstrapped the pool sends one oracle token, one reward token and one ballot token to the new oracle's address. On bootstrap, X oracle and reward tokens are sent to the bootstrapping oracle's address, where X is the total oracle token quantity minted.
The tokens can be sent from any wallet. [scripts/send_new_oracle.sh](scripts/send_new_oracle.sh) does it through a node's wallet API (`send_new_oracle.sh <node url> <node api key> <recipient> <oracle token id> <reward token id> <ballot token id>`). Unlike the oracle itself, that script needs the node's API key and an unlocked node wallet holding the tokens.
Send the new oracle your `pool_config.yaml` as well.

---

## 8. Joining a running pool

To join an existing pool, one oracle token, one reward token and (for voting on contract updates) one ballot token must be received at the address derived from the key in your oracle config. Place the `pool_config.yaml` you received next to your `oracle_config.yaml` and start the oracle with `oracle-core run`.

---

## 9. Extracting reward tokens

Earned reward tokens accumulate in the oracle box. To send all of them, minus the 1 the contract needs, to an address:

```console
oracle-core extract-reward-tokens <ADDRESS>
```

To print how many reward tokens you can extract (the box total minus the 1 the contract keeps):

```console
oracle-core print-reward-tokens
```

---

## 10. Transferring the oracle token to a new operator

Extract the accumulated reward tokens with `extract-reward-tokens` first, then run:

```console
oracle-core transfer-oracle-token <ADDRESS>
```

Make sure the new address has enough ERG for transaction fees. As with inviting a new oracle, send the new operator your `pool_config.yaml`.

Both `extract-reward-tokens` and `transfer-oracle-token` ask you to type `YES` and then wait for the explorer to report confirmation (up to 20 minutes), so run them in an interactive terminal, not from the systemd unit or a detached container.

---

## 11. Updating the contracts and tokens

Changes to the contract parameters or tokens take three steps:

- `prepare-update` submits a new refresh box with the updated refresh contract;
- `vote-update-pool` submits the oracle's ballot box voting for the change;
- `update-pool` submits the update transaction, which produces a new pool box.

Each step is described below. See also [Updating the epoch length](docs/update_epoch_length.md) for a complete example.

### 11.1 Create a new refresh box with `prepare-update`

Create a YAML file describing the contract parameters to update. [Updating the epoch length](docs/update_epoch_length.md) has an example. Run:

```console
oracle-core prepare-update <YAML file>
```

This generates `pool_config_updated.yaml`, which `update-pool` needs later. The output shows the new pool box contract hash and the reward token amounts for the following ten epochs. The other oracles use those values in `vote-update-pool`.

### 11.2 Vote for the update with `vote-update-pool`

```console
oracle-core vote-update-pool <NEW_POOL_BOX_ADDRESS_HASH_STR> <UPDATE_BOX_CREATION_HEIGHT>
```

Required:

- `<NEW_POOL_BOX_ADDRESS_HASH_STR>`: base16 blake2b hash of the serialized pool box contract for the new pool box.
- `<UPDATE_BOX_CREATION_HEIGHT>`: creation height of the existing update box.

Optional, only when a new reward token is minted:

- `<REWARD_TOKEN_ID_STR>`: base16 reward token id in the new pool box.
- `<REWARD_TOKEN_AMOUNT>`: reward token amount in the pool box at the time the update transaction is committed.

All of them are printed by `prepare-update`.

### 11.3 Update the pool box with `update-pool`

Make sure the `pool_config_updated.yaml` generated by `prepare-update` is in the folder the oracle runs from, then:

```console
oracle-core update-pool
```

with the same optional `<REWARD_TOKEN_ID_STR> <REWARD_TOKEN_AMOUNT>` arguments if a new reward token was minted.

After the update transaction is confirmed, switch to the updated pool config: rename `pool_config_updated.yaml` to `pool_config.yaml` and restart the oracle. Distribute the new `pool_config.yaml` to all the oracles.

### 11.4 Import the updated pool config with `import-pool-update`

The other oracles apply the new config with:

```console
oracle-core import-pool-update pool_config_updated.yaml
```

This updates `pool_config.yaml`. Restart the oracle afterwards.

Run it only after the update transaction is confirmed; it refuses while the old pool box still exists. If the update minted a new reward token, `import-pool-update` always refuses, because the reward tokens in your oracle box would be lost: extract them first ([section 9](#9-extracting-reward-tokens)), then copy `pool_config_updated.yaml` over `pool_config.yaml` by hand.

---

## 12. Running as a systemd daemon

Install the unit file from [systemd/oracle-core.service](systemd/oracle-core.service) and edit its `ExecStart=` line. Three things to get right:

- Use absolute paths. systemd does not expand `~`; use the `%h` specifier for your home folder. The unit as shipped uses `~` and a `-c` flag that the current CLI does not have, so it must be edited before it works.
- Point at both config files and a data folder, for example:
  ```ini
  ExecStart=/usr/local/bin/oracle-core --oracle-config-file %h/.config/oracle-core/oracle_config.yaml --pool-config-file %h/.config/oracle-core/pool_config.yaml -d %h/.config/oracle-core run
  EnvironmentFile=%h/.config/oracle-core/oracle.env
  ```
- Give it the mnemonic through `EnvironmentFile=`. systemd ignores lines that start with `export`, so that file must contain the bare assignment `ORACLE_WALLET_MNEMONIC="your mnemonic"`; the `.secrets` file from [3.1](#31-keeping-the-mnemonic-out-of-the-shell-history) only works here if you remove the `export`. `chmod 600` it.


```console
cp systemd/oracle-core.service ~/.config/systemd/user/oracle-core.service
systemctl --user enable oracle-core.service
```

---

## 13. Verifying the contracts against EIP-23

Check that the contracts in use come from EIP-23. Print the encoded hash of each contract with:

```console
./oracle-core print-contract-hashes
```

Like every command, this needs a valid `oracle_config.yaml`, the mnemonic in the environment and a reachable node with the indexer active, because the oracle checks the node before running any subcommand. To check the hashes without a node, use the test instead:

```console
cargo test check_contract_hashes -- --nocapture
```

Either way, compare the values with those in EIP-23.

---

## 14. Metrics

Prometheus metrics are off by default. Set `metrics_port` in the oracle config to enable them. A Grafana dashboard is in the `scripts` folder.

---

## 15. Ubuntu install instructions

A complete walkthrough for running the oracle directly on Ubuntu 24.04, without Docker. It repeats some of the reference material above so you can follow it top to bottom.

### 15.1 Install prerequisites

Oracle Core is written in Rust. You must install the Rust toolchain and a few system libraries.

- System dependencies:
  ```bash
  sudo apt update
  sudo apt install build-essential libssl-dev pkg-config git
  ```
- Rust toolchain, via [rustup.rs](https://rustup.rs):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
  ```
- Check Rust is installed:
  ```bash
  rustc --version
  ```
  The repository pins its toolchain version in `rust-toolchain`; rustup installs it automatically on the first build.

### 15.2 Clone and build Oracle Core

- Clone the repository:
  ```bash
  git clone https://github.com/ergoplatform/oracle-core
  cd oracle-core
  ```
- Build the binary:
  ```bash
  cargo build --release
  ```
- The compiled binary is at `./target/release/oracle-core`. To have `oracle-core` on your PATH instead, run `cargo install --path core`.

### 15.3 Set up Oracle Core

1) Pick the folder the oracle will run from (the rest of this walkthrough assumes `~/oracle-core`), `cd` into it and generate an oracle config file from the default template:
   ```bash
   oracle-core generate-oracle-config
   ```

2) Edit the `oracle_config.yaml` it created and set these parameters:
   - `node_url`: the URL of your Ergo node, for example `http://127.0.0.1:9053/`. The node must run with `ergo.node.extraIndex = true`. No node API key is needed, and the node's wallet is not used.
   - `oracle_network`: `mainnet`
   - `oracle_secret`: `null`
   - `oracle_mnemonic`: `null`  *by leaving both `null`, you make sure the application does **not** look for a key inside this file. Instead it reads the environment variable you set in step 3. The oracle address is derived from that mnemonic, so the mnemonic must be the one whose first address (EIP-3 path `m/44'/429'/0'/0/0`, no passphrase) holds your oracle token, reward token and ERG for fees.*
   - `change_address`: leave it `null` unless you want change from the oracle's transactions sent somewhere other than the oracle address.
   - `metrics_port`: a port such as `9011` IF you plan on installing Grafana to view the prebuilt oracle status dashboard. Otherwise leave it `null`.
   - `min_price_sources`: how many price sources must answer before a datapoint is posted (`erg_usd`, default 3; `gold_usd` for the ERG/XAU pool, default 2). Leave it out for the defaults.
   - `livecoinwatch_api_key`: optional. A free key from livecoinwatch.com adds LiveCoinWatch as an ERG/USD source; without it the other sources are used.

   Any other key from older instructions, such as `oracle_address` or `node_api_key`, is ignored.

3) How to use the `.secrets` file (the recommended method)
   To avoid putting your mnemonic in a terminal command or your shell history, keep it in a file that only your shell reads:
   - Create the hidden secrets file in the same folder:
     ```bash
     nano .secrets
     ```
   - Inside the file, type your mnemonic as an export command:
     ```bash
     export ORACLE_WALLET_MNEMONIC="your twelve or fifteen or however many word mnemonic phrase here"
     ```
     **Save and exit (Ctrl+X, then Y)**
   - Make the file readable only by you:
     ```bash
     chmod 600 .secrets
     ```
   - Load the secrets into your environment:
     ```bash
     source .secrets
     ```
     *Note: you need to run this command in each new terminal and after every reboot.*

4) Obtain a pool configuration. *If joining an existing pool*, contact the pool operator to get their `pool_config.yaml` and place it in the same `~/oracle-core` folder. *If you are creating a pool*, see [Bootstrapping a new oracle pool](#6-bootstrapping-a-new-oracle-pool); the bootstrap writes `pool_config.yaml` for you.

5) Start the oracle. *Load your secrets* from step 3 first, `cd` to the folder and run:
   ```bash
   source .secrets
   oracle-core run
   ```
   IF everything is correct you will see a few startup lines: the version, the config files being loaded, and
   - on the second or third line, `Oracle Address derived from secret: 9xxxxxx` <<< make sure this is the address that holds your oracle token (otherwise you did not enter your mnemonic properly).

   After those lines the oracle goes quiet. That is normal: at the default log level it only prints a line when it posts a datapoint or submits a refresh, and after a restart it may wait a few blocks for its turn (see [What to expect in the logs](#41-what-to-expect-in-the-logs)). The first `Datapoint tx published` line, with a link to the transaction, confirms it is working.

6) Keep it running. Closing the terminal stops the oracle. Use the [systemd unit](#12-running-as-a-systemd-daemon), or a `tmux`/`screen` session, so it survives logouts.
