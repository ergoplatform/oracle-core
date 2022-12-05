# How I Bootsrapped an ERG/XAU Pool on testnet

## Before you start

### Plan pool parameters

Let's say we have 5 operators. We want to keep consensus above 1/2, so it means we can start a pool of 9 oracles (`oracle_tokens:quantity: 9`, `ballot_tokens:quantity: 9`), with 5 oracles threshold for minimum data points (`min_data_points: 5`) and voting (`min_votes: 5`). This way, we'll have 3 vacant oracles places in case someone wants to join later.

## Step 1. Generate a bootstrap config template

Run

```console
oracle-core bootstrap --generate-config-template bootstrap.yaml
```

## Step 2. Edit your bootstrap config template

I made the following changes:

- Set the parameters described in [Plan pool parameters](#plan-pool-parameters)
- Name the tokens in `tokens_to_mint` section.
- Set your node credentials in `node_*` parameters.
- Set data point source `data_point_source: NanoErgXau`
- Set `oracle_address` to my node's wallet address (make sure you have coins).

So in the end, it looked like - <https://gist.github.com/greenhat/2c6135462fba48773196ad45dd6c7404>

## Step 3. Run `bootstrap` command

Run

```console
oracle-core bootstrap bootstrap.yaml
```

It submitted the txs to mint the tokens and make pool, refresh, update boxes. Besides that, it created `oracle_config.yaml` config file to run an oracle.

## Step 4. Invite other operators

To invite other operators, I'm sending one oracle, reward, and ballot tokens to the operator's oracle addresses. I'm using <https://github.com/ergoplatform/oracle-core/blob/develop/scripts/send_new_oracle.sh> for this task.

## Step 5. Start your oracle

I started my oracle with the following:

```console
oracle-core run
```

And it posted the first data point.

## Step 6. Send oracle config to the operators

I made an `oracle_config.yaml` template with

```console
oracle-core print-safe-config > template.yaml
```

and uploaded it to <https://gist.github.com/greenhat/9a0012de5daceac7fa8f6e16c8d11d0f>

I asked the operators to set `node_ip`, `node_port`, `node_api_key`, `oracle_address` to their values and put the file as `oracle_config.yaml` in the same folder and start the oracle with

```console
oracle-core run
```
