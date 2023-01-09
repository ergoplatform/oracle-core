# How I bootsrapped an ERG/XAU pool on testnet

## Before you start

### Plan pool parameters

Let's say we have 5 operators. We want to keep consensus above 1/2, so it means we can start a pool of 9 oracles (`oracle_tokens:quantity: 9`, `ballot_tokens:quantity: 9`), with 5 oracles threshold for minimum data points (`min_data_points: 5`) and voting (`min_votes: 5`). This way, we'll have 3 vacant oracles places in case someone wants to join later.

## Step 1. Generate a bootstrap config template

Generate an oracle config file from the default template with:

```console
oracle-core generate-oracle-config
```

and set the required parameters:

- `oracle_address` to my node's wallet address (make sure you have coins).
- `node_url`, `node_api_key` - node connection parameters;

Run

```console
oracle-core bootstrap --generate-config-template bootstrap.yaml
```

## Step 2. Edit your bootstrap config template

I made the following changes:

- Set the parameters described in [Plan pool parameters](#plan-pool-parameters)
- Name the tokens in `tokens_to_mint` section.
- Set data point source `data_point_source: NanoErgXau`

So in the end, it looked like - <https://gist.github.com/greenhat/2c6135462fba48773196ad45dd6c7404> (old version, before oracle/pool split configs)

## Step 3. Run `bootstrap` command

Run

```console
oracle-core bootstrap bootstrap.yaml
```

It submitted the txs to mint the tokens and make pool, refresh, update boxes. Besides that, it created `pool_config.yaml` config file to run an oracle.

## Step 4. Invite other operators

To invite other operators, I'm sending one oracle, reward, and ballot tokens to the operator's oracle addresses. I'm using <https://github.com/ergoplatform/oracle-core/blob/develop/scripts/send_new_oracle.sh> for this task.

## Step 5. Start your oracle

I started my oracle with the following:

```console
oracle-core run
```

And it posted the first data point.

## Step 6. Send pool config to the operators

Besides the tokens the pool config file that you are running now should be sent as well. Send `pool_config.yaml` to the operators and ask them to start the oracle with

```console
oracle-core run
```

After they start their oracles keep an eye on your oracle log file and wait for refresh tx generated. It means your pool is running and the pool box was updated.
