use std::{
    convert::TryFrom,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;
use ergo_lib::wallet::ext_secret_key::ExtSecretKey;
use ergo_lib::wallet::mnemonic::Mnemonic;
use ergo_lib::wallet::secret_key::SecretKey;
use ergo_lib::{
    ergotree_ir::chain::address::NetworkAddress,
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoder},
            ergo_box::box_value::BoxValue,
        },
        sigma_protocol::sigma_boolean::ProveDlog,
    },
    wallet::tx_builder::{self, SUGGESTED_TX_FEE},
};
use log::LevelFilter;
use once_cell::sync;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::explorer_api::explorer_url::default_explorer_api_url;

pub const DEFAULT_ORACLE_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OracleConfig {
    pub node_url: Url,
    pub base_fee: u64,
    pub log_level: Option<LevelFilter>,
    pub core_api_port: u16,
    pub oracle_address: NetworkAddress,
    pub change_address: Option<NetworkAddress>,
    pub data_point_source_custom_script: Option<String>,
    pub explorer_url: Option<Url>,
    pub metrics_port: Option<u16>,
}

pub struct OracleSecrets {
    pub secret_key: SecretKey,
}

impl OracleSecrets {
    pub fn load() -> Self {
        let mnemonic = std::env::var("ORACLE_WALLET_MNEMONIC").unwrap_or_else(|_| {
            panic!("ORACLE_WALLET_MNEMONIC environment variable for sign transactions is not set")
        });

        let seed = Mnemonic::to_seed(&mnemonic, "");
        let ext_sk = ExtSecretKey::derive_master(seed).unwrap();
        // bip-32 path for the first key
        let path = "m/44'/429'/0'/0/0";
        let secret = ext_sk.derive(path.parse().unwrap()).unwrap().secret_key();

        Self { secret_key: secret }
    }
}

impl OracleConfig {
    pub fn write_default_config_file(path: &Path) {
        let config = OracleConfig::default();
        let yaml_str = serde_yaml::to_string(&config).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(yaml_str.as_bytes()).unwrap();
    }

    fn load() -> Result<Self, anyhow::Error> {
        let config_file_path = ORACLE_CONFIG_FILE_PATH.get().ok_or_else(|| {
            OracleConfigFileError::IoError("ORACLE_CONFIG_FILE_PATH not set".to_string())
        })?;
        log::info!("Loading oracle config from {}", config_file_path.display());
        let config_str = std::fs::read_to_string(config_file_path).context(format!(
            "failed to load oracle config file from {}",
            config_file_path.display()
        ))?;
        let mut config =
            Self::load_from_str(&config_str).context("failed to parse oracle config file")?;
        if config.change_address.is_none() {
            config.change_address = Some(config.oracle_address.clone());
            log::info!("Set oracle address as change address");
        }
        let _ = config
            .oracle_address_p2pk()
            .context("failed to parse oracle address")?;

        let _ = config
            .change_address_p2pk()
            .context("failed to parse change address")?;
        Ok(config.clone())
    }

    pub fn load_from_str(config_str: &str) -> Result<Self, OracleConfigFileError> {
        serde_yaml::from_str(config_str)
            .map_err(|e| OracleConfigFileError::ParseError(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<(), OracleConfigFileError> {
        let yaml_str = serde_yaml::to_string(self).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(yaml_str.as_bytes()).unwrap();
        Ok(())
    }

    pub fn oracle_address_p2pk(&self) -> Result<ProveDlog, OracleConfigFileError> {
        if let Address::P2Pk(public_key) = self.oracle_address.address() {
            Ok(public_key.clone())
        } else {
            Err(OracleConfigFileError::InvalidOracleAddress)
        }
    }

    pub fn change_address_p2pk(&self) -> Result<ProveDlog, OracleConfigFileError> {
        if let Address::P2Pk(public_key) = self.change_address.clone().unwrap().address() {
            Ok(public_key.clone())
        } else {
            Err(OracleConfigFileError::InvalidChangeAddress)
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum OracleConfigFileError {
    #[error("Error reading oracle config file: {0}")]
    IoError(String),
    #[error("Error parsing oracle config file: {0}")]
    ParseError(String),
    #[error("Invalid oracle address, must be P2PK")]
    InvalidOracleAddress,
    #[error("Invalid change address, must be P2PK")]
    InvalidChangeAddress,
}

impl Default for OracleConfig {
    fn default() -> Self {
        let address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9hEQHEMyY1K1vs79vJXFtNjr2dbQbtWXF99oVWGJ5c4xbcLdBsw",
        )
        .unwrap();
        Self {
            oracle_address: address.clone(),
            change_address: None,
            core_api_port: 9010,
            data_point_source_custom_script: None,
            base_fee: *tx_builder::SUGGESTED_TX_FEE().as_u64(),
            log_level: LevelFilter::Info.into(),
            node_url: Url::parse("http://127.0.0.1:9053").unwrap(),
            explorer_url: Some(default_explorer_api_url(address.network())),
            metrics_port: None,
        }
    }
}

pub static ORACLE_CONFIG_FILE_PATH: sync::OnceCell<PathBuf> = sync::OnceCell::new();
lazy_static! {
    pub static ref ORACLE_CONFIG: OracleConfig = OracleConfig::load().unwrap();
    pub static ref ORACLE_SECRETS: OracleSecrets = OracleSecrets::load();
    pub static ref ORACLE_CONFIG_OPT: Result<OracleConfig, anyhow::Error> = OracleConfig::load();
    pub static ref BASE_FEE: BoxValue = ORACLE_CONFIG_OPT
        .as_ref()
        .map(|c| BoxValue::try_from(c.base_fee).unwrap())
        .unwrap_or_else(|_| SUGGESTED_TX_FEE());
}
