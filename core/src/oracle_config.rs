use std::{
    convert::TryFrom,
    io::Write,
    path::{Path, PathBuf},
};

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
    pub node_api_key: String,
    pub base_fee: u64,
    pub log_level: Option<LevelFilter>,
    pub core_api_port: u16,
    pub oracle_address: NetworkAddress,
    pub data_point_source_custom_script: Option<String>,
    pub explorer_url: Option<Url>,
}

impl OracleConfig {
    pub fn write_default_config_file(path: &Path) {
        let config = OracleConfig::default();
        let yaml_str = serde_yaml::to_string(&config).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(yaml_str.as_bytes()).unwrap();
    }

    fn load() -> Result<Self, OracleConfigFileError> {
        let config_file_path = ORACLE_CONFIG_FILE_PATH.get().ok_or_else(|| {
            OracleConfigFileError::IoError("ORACLE_CONFIG_FILE_PATH not set".to_string())
        })?;
        let config_str: &str = &std::fs::read_to_string(config_file_path)
            .map_err(|e| OracleConfigFileError::IoError(e.to_string()))?;
        let config = Self::load_from_str(config_str)?;
        let _ = config.oracle_address_p2pk()?;
        Ok(config)
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
}

#[derive(Clone, Debug, Error)]
pub enum OracleConfigFileError {
    #[error("Error reading oracle config file: {0}")]
    IoError(String),
    #[error("Error parsing oracle config file: {0}")]
    ParseError(String),
    #[error("Invalid oracle address, must be P2PK")]
    InvalidOracleAddress,
}

impl Default for OracleConfig {
    fn default() -> Self {
        let address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9hEQHEMyY1K1vs79vJXFtNjr2dbQbtWXF99oVWGJ5c4xbcLdBsw",
        )
        .unwrap();
        Self {
            oracle_address: address.clone(),
            node_api_key: "hello".into(),
            core_api_port: 9010,
            data_point_source_custom_script: None,
            base_fee: *tx_builder::SUGGESTED_TX_FEE().as_u64(),
            log_level: LevelFilter::Info.into(),
            node_url: Url::parse("http://127.0.0.1:9053").unwrap(),
            explorer_url: Some(default_explorer_api_url(address.network())),
        }
    }
}

pub static ORACLE_CONFIG_FILE_PATH: sync::OnceCell<PathBuf> = sync::OnceCell::new();
lazy_static! {
    pub static ref ORACLE_CONFIG: OracleConfig = OracleConfig::load().unwrap();
    pub static ref ORACLE_CONFIG_OPT: Result<OracleConfig, OracleConfigFileError> =
        OracleConfig::load();
    pub static ref BASE_FEE: BoxValue = ORACLE_CONFIG_OPT
        .as_ref()
        .map(|c| BoxValue::try_from(c.base_fee).unwrap())
        .unwrap_or_else(|_| SUGGESTED_TX_FEE());
}

