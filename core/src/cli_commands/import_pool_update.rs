use std::path::Path;

use anyhow::anyhow;

use crate::box_kind::OracleBox;
use crate::oracle_state::LocalDatapointBoxSource;
use crate::pool_config::PoolConfig;
use crate::pool_config::POOL_CONFIG;
use crate::spec_token::OracleTokenId;
use crate::spec_token::RewardTokenId;

pub fn import_pool_update(
    new_pool_config_file: String,
    oracle_token_id: &OracleTokenId,
    reward_token_id: &RewardTokenId,
    current_pool_config_path: &Path,
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
    scan_ids_path: &Path,
) -> Result<(), anyhow::Error> {
    let new_pool_config_str =
        std::fs::read_to_string(new_pool_config_file.clone()).map_err(|e| {
            anyhow!(
                "Failed to read pool config from file {:?}: {}",
                new_pool_config_file,
                e
            )
        })?;
    let new_pool_config = PoolConfig::load_from_str(&new_pool_config_str).map_err(|e| {
        anyhow!(
            "Failed to parse pool config from file {:?}: {}",
            new_pool_config_file,
            e
        )
    })?;
    if &new_pool_config.token_ids.oracle_token_id != oracle_token_id {
        let in_oracle_box = local_datapoint_box_source
            .get_local_oracle_datapoint_box()
            .map_err(|e| anyhow!("Failed to get local oracle datapoint box: {}", e))?
            .unwrap();
        let num_reward_tokens = *in_oracle_box.reward_token().amount.as_u64();
        if num_reward_tokens > 1 {
            return Err(
                anyhow!("Since new oracle token is minted reward tokens from the current oracle box will be lost. Please transfer them to a different address with extract-reward-tokens command before importing new pool config.")
            );
        }
    }
    if &new_pool_config.token_ids.reward_token_id != reward_token_id {
        return Err(
                anyhow!("Since new reward token is minted reward tokens from the current oracle box will be lost. Please transfer them to a different address with extract-reward-tokens command before importing new pool config.")
            );
    }

    let new_token_ids = &new_pool_config.token_ids;
    let old_token_ids = &POOL_CONFIG.token_ids;
    if new_token_ids.pool_nft_token_id != old_token_ids.pool_nft_token_id
        || new_token_ids.refresh_nft_token_id != old_token_ids.refresh_nft_token_id
        || new_token_ids.oracle_token_id != old_token_ids.oracle_token_id
        || new_token_ids.update_nft_token_id != old_token_ids.update_nft_token_id
        || new_token_ids.ballot_token_id != old_token_ids.ballot_token_id
    {
        // TODO: get scans from NodeScanRegistry
        // TODO: unregister scans with node
        std::fs::remove_file(scan_ids_path)
            .map_err(|e| anyhow!("Failed to remove scan ids file {:?}: {}", scan_ids_path, e))?;
    }
    new_pool_config.save(current_pool_config_path)?;
    Ok(())
}
