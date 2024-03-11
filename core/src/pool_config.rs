use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use once_cell::sync;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::box_kind::BallotBoxWrapperInputs;
use crate::box_kind::OracleBoxWrapperInputs;
use crate::box_kind::PoolBoxWrapperInputs;
use crate::box_kind::RefreshBoxWrapperInputs;
use crate::box_kind::UpdateBoxWrapperInputs;
use crate::cli_commands::bootstrap::BootstrapConfig;
use crate::contracts::ballot::BallotContractError;
use crate::contracts::oracle::OracleContractError;
use crate::contracts::pool::PoolContractError;
use crate::contracts::refresh::RefreshContractError;
use crate::contracts::update::UpdateContractError;
use crate::spec_token::BallotTokenId;
use crate::spec_token::BuybackTokenId;
use crate::spec_token::OracleTokenId;
use crate::spec_token::PoolTokenId;
use crate::spec_token::RefreshTokenId;
use crate::spec_token::RewardTokenId;
use crate::spec_token::UpdateTokenId;

pub const DEFAULT_POOL_CONFIG_FILE_NAME: &str = "pool_config.yaml";
pub static POOL_CONFIG_FILE_PATH: sync::OnceCell<PathBuf> = sync::OnceCell::new();
lazy_static! {
    pub static ref POOL_CONFIG: PoolConfig = PoolConfig::load().unwrap();
    pub static ref POOL_CONFIG_OPT: Result<PoolConfig, String> =
        PoolConfig::load().map_err(|e| e.to_string());
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    try_from = "crate::serde::PoolConfigSerde",
    into = "crate::serde::PoolConfigSerde"
)]
pub struct PoolConfig {
    pub data_point_source: Option<PredefinedDataPointSource>,
    pub oracle_box_wrapper_inputs: OracleBoxWrapperInputs,
    pub pool_box_wrapper_inputs: PoolBoxWrapperInputs,
    pub refresh_box_wrapper_inputs: RefreshBoxWrapperInputs,
    pub update_box_wrapper_inputs: UpdateBoxWrapperInputs,
    pub ballot_box_wrapper_inputs: BallotBoxWrapperInputs,
    pub token_ids: TokenIds,
    pub buyback_token_id: Option<BuybackTokenId>,
    pub dev_reward_ergo_tree_bytes: Option<String>
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum PredefinedDataPointSource {
    NanoErgUsd,
    NanoErgXau,
    NanoAdaUsd,
    NanoErgBTC,
}

/// Holds the token ids of every important token used by the oracle pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenIds {
    #[serde(
        serialize_with = "crate::serde::token_id_as_base16_string",
        deserialize_with = "crate::serde::token_id_from_base16"
    )]
    pub pool_nft_token_id: PoolTokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base16_string",
        deserialize_with = "crate::serde::token_id_from_base16"
    )]
    pub refresh_nft_token_id: RefreshTokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base16_string",
        deserialize_with = "crate::serde::token_id_from_base16"
    )]
    pub update_nft_token_id: UpdateTokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base16_string",
        deserialize_with = "crate::serde::token_id_from_base16"
    )]
    pub oracle_token_id: OracleTokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base16_string",
        deserialize_with = "crate::serde::token_id_from_base16"
    )]
    pub reward_token_id: RewardTokenId,
    #[serde(
        serialize_with = "crate::serde::token_id_as_base16_string",
        deserialize_with = "crate::serde::token_id_from_base16"
    )]
    pub ballot_token_id: BallotTokenId,
}

#[derive(Debug, Error)]
pub enum PoolConfigError {
    #[error("Oracle contract error: {0}")]
    OracleContractError(#[from] OracleContractError),
    #[error("Refresh contract error: {0}")]
    RefreshContractError(#[from] RefreshContractError),
    #[error("Pool contract error: {0}")]
    PoolContractError(#[from] PoolContractError),
    #[error("Update contract error: {0}")]
    UpdateContractErro(#[from] UpdateContractError),
    #[error("Ballot contract error: {0}")]
    BallotContractErro(#[from] BallotContractError),
}

impl PoolConfig {
    pub fn create(
        bootstrap: BootstrapConfig,
        token_ids: TokenIds,
    ) -> Result<Self, PoolConfigError> {
        let oracle_box_wrapper_inputs = OracleBoxWrapperInputs::build_with(
            bootstrap.oracle_contract_parameters.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.oracle_token_id.clone(),
            token_ids.reward_token_id.clone(),
        )?;
        let refresh_box_wrapper_inputs = RefreshBoxWrapperInputs::build_with(
            bootstrap.refresh_contract_parameters.clone(),
            token_ids.oracle_token_id.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.refresh_nft_token_id.clone(),
        )?;
        let pool_box_wrapper_inputs = PoolBoxWrapperInputs::build_with(
            bootstrap.pool_contract_parameters.clone(),
            token_ids.refresh_nft_token_id.clone(),
            token_ids.update_nft_token_id.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.reward_token_id.clone(),
        )?;
        let update_box_wrapper_inputs = UpdateBoxWrapperInputs::build_with(
            bootstrap.update_contract_parameters.clone(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.ballot_token_id.clone(),
            token_ids.update_nft_token_id.clone(),
        )?;
        let ballot_box_wrapper_inputs = BallotBoxWrapperInputs::build_with(
            bootstrap.ballot_contract_parameters.clone(),
            token_ids.ballot_token_id.clone(),
            token_ids.update_nft_token_id.clone(),
        )?;
        Ok(PoolConfig {
            data_point_source: bootstrap.data_point_source,
            oracle_box_wrapper_inputs,
            pool_box_wrapper_inputs,
            refresh_box_wrapper_inputs,
            ballot_box_wrapper_inputs,
            update_box_wrapper_inputs,
            token_ids,
            buyback_token_id: None,
            dev_reward_ergo_tree_bytes: None,
        })
    }

    fn load() -> Result<Self, anyhow::Error> {
        let config_file_path = POOL_CONFIG_FILE_PATH
            .get()
            .ok_or_else(|| anyhow!("Pool config file path not set"))?;
        log::info!("Loading pool config from {}", config_file_path.display());
        let config_str = std::fs::read_to_string(config_file_path).context(format!(
            "failed to load pool config file from {}",
            config_file_path.display()
        ))?;
        Self::load_from_str(&config_str)
    }

    pub fn save(&self, path: &Path) -> Result<(), anyhow::Error> {
        let yaml_str = serde_yaml::to_string(self).unwrap();
        std::fs::write(path, yaml_str)?;
        Ok(())
    }

    pub fn load_from_str(config_str: &str) -> Result<PoolConfig, anyhow::Error> {
        serde_yaml::from_str(config_str).context("failed to parse pool config file")
    }
}

#[cfg(test)]
mod tests {

    use crate::pool_commands::test_utils::generate_token_ids;

    use super::*;

    #[test]
    fn token_ids_roundtrip() {
        let token_ids = generate_token_ids();
        let s = serde_yaml::to_string(&token_ids).unwrap();
        assert_eq!(token_ids, serde_yaml::from_str::<TokenIds>(&s).unwrap());
    }
}
