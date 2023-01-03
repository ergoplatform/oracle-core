use std::path::Path;

use crate::oracle_config::OracleConfig;
use crate::pool_config::PoolConfig;
use anyhow::anyhow;

pub fn try_migrate_to_split_config(
    oracle_config_path: &Path,
    pool_config_path: &Path,
) -> Result<(), anyhow::Error> {
    let oracle_config_str = std::fs::read_to_string(oracle_config_path).map_err(|e| {
        anyhow!(
            "Failed to read oracle config file at path {:?}: {}",
            oracle_config_path,
            e
        )
    })?;
    // if the pool config cannot be loaded it means
    // we might have a new oracle config without a bootstrapped pool
    // in this case we exit silently and skip the migration
    if let Ok(pool_config) = PoolConfig::load_from_str(&oracle_config_str) {
        println!(
            "pool_config.yaml not found, using oracle_config.yaml for migration to split config"
        );
        let oracle_config = OracleConfig::load_from_str(&oracle_config_str).map_err(|e| {
            anyhow!(
                "Failed to parse oracle config file at path {:?}: {}",
                oracle_config_path,
                e
            )
        })?;
        pool_config.save(pool_config_path).map_err(|e| {
            anyhow!(
                "Failed to save pool config file at path {:?}: {}",
                pool_config_path,
                e
            )
        })?;

        oracle_config.save(oracle_config_path).map_err(|e| {
            anyhow!(
                "Failed to save(overwrite) oracle config file at path {:?}: {}",
                oracle_config_path,
                e
            )
        })?;
    };
    Ok(())
}
