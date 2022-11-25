## How to update epoch length

### Create a new refresh box with `prepare-update` command 
Create a file `prepare_update_conf.yaml`:
```yaml
---
refresh_contract_parameters:
  ergo_tree_bytes: 1016043c040004000e20c7e3691bce1ca08125c9ecb0960d4d71c8efd4db876910620864db701d8cc99c01000502010105000400040004020402040204040400040a05c8010e20bf1c100d3fa68b9eb486d6032d4b2e3d6abe3952faeb8a999877e6cf517b337e0400040404020408d80ed60199a37300d602b2a4730100d603b5a4d901036395e6c672030605eded928cc77203017201938cb2db6308720373020001730393e4c672030504e4c6720205047304d604b17203d605b0720386027305860273067307d901053c413d0563d803d607e4c68c7205020605d6088c720501d6098c720802860272078602ed8c720901908c72080172079a8c7209027207d6068c720502d6078c720501d608db63087202d609b27208730800d60ab2a5730900d60bdb6308720ad60cb2720b730a00d60db27208730b00d60eb2a5730c00ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02cde4c6b27203e4e30004000407d18f8cc77202017201d1927204730dd18c720601d190997207e4c6b27203730e0006059d9c72077e730f057310d1938c7209017311d193b2720b7312007209d1938c720c018c720d01d1928c720c02998c720d027e9c7204731305d193b1720bb17208d193e4c6720a04059d8c7206027e720405d193e4c6720a05049ae4c6720205047314d193c2720ac27202d192c1720ac17202d1928cc7720a0199a37315d193db6308720edb6308a7d193c2720ec2a7d192c1720ec1a7
  pool_nft_index: 17
  oracle_token_id_index: 3
  min_data_points_index: 13
  min_data_points: 2
  buffer_length_index: 21
  buffer_length: 4
  max_deviation_percent_index: 15
  max_deviation_percent: 5
  epoch_length_index: 0
  epoch_length: 20
pool_contract_parameters:
  ergo_tree_bytes: 1004040204000e204774b39aaf9bd8e62751e6d789e856bc26d6d33b8a72379dcb735c0ac21584650e20729447a36d774520cdda2dd6730e0ba3f7ce2161e253fa85c50ab1547e7b909cd801d6018cb2db6308b2a473000073010001d1ec93720173029372017303
  refresh_nft_index: 2
  update_nft_index: 3
tokens_to_mint:
  pool_nft: ~
  refresh_nft:
    name: refresh NFT
    description: refresh NFT
  update_nft: ~
  oracle_tokens: ~
  ballot_tokens: ~
  reward_tokens: ~
```
Where `refresh_contract_parameters` section is copied from your current `oracle_config.yaml` with `epoch_lengh` changed to the desired value (to 20 in this case), `pool_contract_parameters` copied without changes, `tokens_to_mint` is copied from bootstrap config with only `refresh_nft` set to a desired values (new refresh NFT will be minted for the new refresh contract and will be used in the new pool box contract).

Then run:
```console
oracle-core prepare-update prepare_update_conf.yaml
```
and check that `oracle_config_updated.yaml` with new refresh contract, pool contract and refresh NFT is generated.
The new pool contract hash is printed along with current reward token amount and guesstimated reward token amounts for the upcoming epochs.

### Vote for the change with `vote-update-pool` command.
Run
```console
oracle-core vote-update-pool <NEW_POOL_BOX_ADDRESS_HASH_STR> <REWARD_TOKEN_ID_STR> <REWARD_TOKEN_AMOUNT> <UPDATE_BOX_CREATION_HEIGHT>
```
Where:
- <NEW_POOL_BOX_ADDRESS_HASH_STR> - base16-encoded blake2b hash of the serialized pool box contract for the new pool box
- <REWARD_TOKEN_ID_STR> - base16-encoded reward token id in the new pool box (use existing if unchanged)
- <REWARD_TOKEN_AMOUNT> - reward token amount in the pool box at the time of update transaction is committed
- <UPDATE_BOX_CREATION_HEIGHT> - The creation height of the existing update box.

are printed in the output of the `prepare-update` command. 

Keep in mind the REWARD_TOKEN_AMOUNT depends on when(in which epoch) the final `update-pool` command will be run.

### Commit the update to the pool box contract with `update-pool` command.
Make sure the `oracle_config_updated.yaml` config file generated during the `prepare-update` command is in the same folder as the oracle-core binary.
Run
```console
oracle-core update-pool <NEW_POOL_BOX_ADDRESS_HASH_STR> <REWARD_TOKEN_ID_STR> <REWARD_TOKEN_AMOUNT> 
```
Where:
  <NEW_POOL_BOX_ADDRESS_HASH_STR> - base16-encoded blake2b hash of the serialized pool box contract for the new pool box
  <REWARD_TOKEN_ID_STR> - base16-encoded reward token id in the new pool box (use existing if unchanged)
  <REWARD_TOKEN_AMOUNT> - reward token amount in the pool box at the time of update transaction is committed

were printed at the end of the `prepare-update` command.

This will submit an update tx. 
After the update tx is confirmed, use `oracle_config_updated.yaml` to run the oracle (i.e., rename it to `oracle_config.yaml` and restart the oracle)
