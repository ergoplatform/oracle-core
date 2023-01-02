use crate::oracle_config::OracleConfig;
use crate::oracle_config::ORACLE_CONFIG_FILE_PATH;
use crate::pool_config::PoolConfig;
use crate::pool_config::POOL_CONFIG_FILE_PATH;
use anyhow::anyhow;

pub fn migrate_to_split_config() -> Result<(), anyhow::Error> {
    let oracle_file_path = &ORACLE_CONFIG_FILE_PATH.get().unwrap();
    let oracle_config_str = std::fs::read_to_string(oracle_file_path).map_err(|e| {
        anyhow!(
            "Failed to read oracle config file at path {:?}: {}",
            oracle_file_path,
            e
        )
    })?;
    let pool_config = PoolConfig::load_from_str(&oracle_config_str).map_err(|e| {
        anyhow!(
            "Failed to parse pool config file at path {:?}: {}",
            oracle_file_path,
            e
        )
    })?;

    let oracle_config = OracleConfig::load().map_err(|e| {
        anyhow!(
            "Failed to parse oracle config file at path {:?}: {}",
            oracle_file_path,
            e
        )
    })?;
    pool_config.save().map_err(|e| {
        anyhow!(
            "Failed to save pool config file at path {:?}: {}",
            POOL_CONFIG_FILE_PATH,
            e
        )
    })?;

    oracle_config.save().map_err(|e| {
        anyhow!(
            "Failed to save(overwrite) oracle config file at path {:?}: {}",
            oracle_file_path,
            e
        )
    })?;
    Ok(())
}
