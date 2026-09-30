use std::{
    convert::TryFrom,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::wallet::ext_secret_key::ExtSecretKey;
use ergo_lib::wallet::mnemonic::Mnemonic;
use ergo_lib::wallet::secret_key::SecretKey;
use ergo_lib::{
    ergotree_ir::chain::address::NetworkAddress,
    ergotree_ir::{
        chain::{address::Address, ergo_box::box_value::BoxValue},
        sigma_protocol::sigma_boolean::ProveDlog,
    },
    wallet::tx_builder::{self, SUGGESTED_TX_FEE},
};
use log::LevelFilter;
use once_cell::sync;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::datapoint_source::MinPriceSources;
use crate::explorer_api::explorer_url::default_explorer_api_url;

pub const DEFAULT_ORACLE_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OracleConfig {
    pub node_url: Url,
    pub base_fee: u64,
    pub log_level: Option<LevelFilter>,
    pub core_api_port: u16,
    oracle_network: String,
    #[serde(skip)]
    network_prefix: Option<NetworkPrefix>,
    #[serde(skip)]
    oracle_address: Option<NetworkAddress>,
    #[serde(skip)]
    oracle_secret_key: Option<SecretKey>,
    oracle_secret: Option<String>,
    oracle_mnemonic: Option<String>,
    change_address: Option<NetworkAddress>,
    pub data_point_source_custom_script: Option<String>,
    #[serde(default, deserialize_with = "min_price_sources_or_default")]
    pub min_price_sources: MinPriceSources,
    /// LiveCoinWatch (free key from livecoinwatch.com) is an ERG/USD source only
    /// when this is set.
    #[serde(default)]
    pub livecoinwatch_api_key: Option<String>,
    pub explorer_url: Option<Url>,
    pub metrics_port: Option<u16>,
}

/// `min_price_sources: ~` (the style of the other optional keys) means the defaults.
fn min_price_sources_or_default<'de, D>(deserializer: D) -> Result<MinPriceSources, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<MinPriceSources>::deserialize(deserializer)?.unwrap_or_default())
}

impl OracleConfig {
    pub fn write_default_config_file(path: &Path) {
        let config = OracleConfig::default();
        let yaml_str = serde_yaml::to_string(&config).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(yaml_str.as_bytes()).unwrap();
    }

    fn network_prefix(network: &str) -> Result<NetworkPrefix, OracleConfigFileError> {
        match network.to_lowercase().as_str() {
            "mainnet" => Ok(NetworkPrefix::Mainnet),
            "testnet" => Ok(NetworkPrefix::Testnet),
            _ => Err(OracleConfigFileError::InvalidNetwork(network.to_string())),
        }
    }

    /// Returns network prefix (mainnet/testnet) from config
    pub fn get_network_prefix(&self) -> NetworkPrefix {
        self.network_prefix
            .unwrap_or_else(|| Self::network_prefix(&self.oracle_network).unwrap())
    }

    /// Sets oracle address for testing purposes only
    pub fn set_oracle_address(&mut self, oracle_address: Option<NetworkAddress>) {
        self.oracle_address = oracle_address
    }

    /// Returns the oracle address, panics if not set
    pub fn get_oracle_address(&self) -> NetworkAddress {
        self.oracle_address.clone().unwrap()
    }

    /// Returns change address for transactions, panics if not set
    pub fn get_change_address(&self) -> NetworkAddress {
        self.change_address.clone().unwrap()
    }

    /// Returns the oracle's secret key, panics if not set
    pub fn get_oracle_secret_key(&self) -> SecretKey {
        self.oracle_secret_key.clone().unwrap()
    }

    /// Sets oracle secret key from environment or config file
    fn set_mnemonic_secret(&mut self) -> Result<(), OracleConfigFileError> {
        // Try environment variable ORACLE_WALLET_SECRET
        if let Ok(secret) = std::env::var("ORACLE_WALLET_SECRET") {
            let secret_bytes = base16::decode(&secret).map_err(|e| {
                OracleConfigFileError::MnemonicError(format!("Invalid hex format: {}", e))
            })?;

            if let Ok(secret_key) = SecretKey::from_bytes(&secret_bytes) {
                self.oracle_secret_key = Some(secret_key);
                return Ok(());
            }
        }

        // Try config's oracle_secret
        if let Some(secret) = &self.oracle_secret {
            let secret_bytes = base16::decode(secret).map_err(|e| {
                OracleConfigFileError::MnemonicError(format!("Invalid hex format in config: {}", e))
            })?;

            if let Ok(secret_key) = SecretKey::from_bytes(&secret_bytes) {
                self.oracle_secret_key = Some(secret_key);
                return Ok(());
            }
        }

        // Try environment variable ORACLE_WALLET_MNEMONIC
        if let Ok(mnemonic) = std::env::var("ORACLE_WALLET_MNEMONIC") {
            if let Ok(secret_key) = self.derive_secret_from_mnemonic(&mnemonic) {
                self.oracle_secret_key = Some(secret_key);
                return Ok(());
            }
        }

        // Try config's mnemonic
        if let Some(mnemonic) = &self.oracle_mnemonic {
            if let Ok(secret_key) = self.derive_secret_from_mnemonic(mnemonic) {
                self.oracle_secret_key = Some(secret_key);
                return Ok(());
            }
        }

        Err(OracleConfigFileError::MissingMnemonicSecret)
    }

    fn derive_secret_from_mnemonic(
        &self,
        mnemonic: &str,
    ) -> Result<SecretKey, OracleConfigFileError> {
        let seed = Mnemonic::to_seed(mnemonic, "");
        let ext_sk = ExtSecretKey::derive_master(seed)
            .map_err(|e| OracleConfigFileError::MnemonicError(e.to_string()))?;

        // bip-32 path for the first key
        let path = "m/44'/429'/0'/0/0";
        let secret_key = ext_sk
            .derive(path.parse().unwrap())
            .map_err(|e| OracleConfigFileError::MnemonicError(e.to_string()))?
            .secret_key();

        Ok(secret_key)
    }

    /// Derives oracle address from the secret key
    fn derive_oracle_address(&mut self) -> Result<NetworkAddress, OracleConfigFileError> {
        self.set_mnemonic_secret()?;
        let network_prefix = self.get_network_prefix();
        let oracle_address = NetworkAddress::new(
            network_prefix,
            &self
                .oracle_secret_key
                .clone()
                .unwrap()
                .get_address_from_public_image(),
        );
        log::info!(
            "Oracle Address derived from secret: {}",
            oracle_address.to_base58()
        );
        Ok(oracle_address)
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

        // Set network prefix
        config.network_prefix = Some(
            Self::network_prefix(&config.oracle_network)
                .context("failed to parse network prefix")?,
        );

        // Derive oracle address from mnemonic
        config.oracle_address = Some(
            config
                .derive_oracle_address()
                .context("failed to derive oracle address from mnemonic")?,
        );

        if config.change_address.is_none() {
            config.change_address = Some(config.oracle_address.clone().unwrap());
            log::info!("Set oracle address as change address");
        }
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
        if let Address::P2Pk(public_key) = self.get_oracle_address().address() {
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
    #[error("Invalid network: {0}. Must be either 'mainnet' or 'testnet'")]
    InvalidNetwork(String),
    #[error("Mnemonic or Secret not found in environment or config file")]
    MissingMnemonicSecret,
    #[error("Error processing mnemonic: {0}")]
    MnemonicError(String),
}

impl Default for OracleConfig {
    fn default() -> Self {
        let network = "mainnet".to_string();
        let network_prefix = NetworkPrefix::Mainnet;
        Self {
            oracle_address: None,
            oracle_secret: None,
            oracle_secret_key: None,
            oracle_mnemonic: None,
            oracle_network: network.clone(),
            network_prefix: Some(network_prefix),
            change_address: None,
            core_api_port: 9010,
            data_point_source_custom_script: None,
            min_price_sources: MinPriceSources::default(),
            livecoinwatch_api_key: None,
            base_fee: *tx_builder::SUGGESTED_TX_FEE().as_u64(),
            log_level: LevelFilter::Info.into(),
            node_url: Url::parse("http://127.0.0.1:9053").unwrap(),
            explorer_url: Some(default_explorer_api_url(network_prefix)),
            metrics_port: None,
        }
    }
}

pub static ORACLE_CONFIG_FILE_PATH: sync::OnceCell<PathBuf> = sync::OnceCell::new();
lazy_static! {
    pub static ref ORACLE_CONFIG: OracleConfig = OracleConfig::load().unwrap();
    pub static ref ORACLE_CONFIG_OPT: Result<OracleConfig, anyhow::Error> = OracleConfig::load();
    pub static ref BASE_FEE: BoxValue = ORACLE_CONFIG_OPT
        .as_ref()
        .map(|c| BoxValue::try_from(c.base_fee).unwrap())
        .unwrap_or_else(|_| SUGGESTED_TX_FEE());
}

#[cfg(test)]
mod tests {
    use super::*;

    // an oracle_config.yaml written before min_price_sources existed
    const OLD_CONFIG: &str = r#"
node_url: http://127.0.0.1:9053
base_fee: 1100000
log_level: ~
core_api_port: 9010
oracle_network: mainnet
data_point_source_custom_script: ~
explorer_url: ~
metrics_port: ~
"#;

    #[test]
    fn test_old_config_loads_with_default_min_price_sources() {
        let config = OracleConfig::load_from_str(OLD_CONFIG).unwrap();
        assert_eq!(config.min_price_sources, MinPriceSources::default());
        assert_eq!(config.min_price_sources.erg_usd, 3);
        assert_eq!(config.min_price_sources.gold_usd, 2);
    }

    #[test]
    fn test_min_price_sources() {
        let config = OracleConfig::load_from_str(&format!(
            "{OLD_CONFIG}min_price_sources:\n  erg_usd: 5\n  gold_usd: 1\n"
        ))
        .unwrap();
        assert_eq!(
            config.min_price_sources,
            MinPriceSources {
                erg_usd: 5,
                gold_usd: 1
            }
        );
        let config =
            OracleConfig::load_from_str(&format!("{OLD_CONFIG}min_price_sources: ~\n")).unwrap();
        assert_eq!(config.min_price_sources, MinPriceSources::default());
        // a key left out takes its default
        let config =
            OracleConfig::load_from_str(&format!("{OLD_CONFIG}min_price_sources:\n  erg_usd: 4\n"))
                .unwrap();
        assert_eq!(config.min_price_sources.erg_usd, 4);
        assert_eq!(config.min_price_sources.gold_usd, 2);
    }

    #[test]
    fn test_livecoinwatch_api_key() {
        // absent (an older config), null, empty and set
        let config = OracleConfig::load_from_str(OLD_CONFIG).unwrap();
        assert_eq!(config.livecoinwatch_api_key, None);
        let config =
            OracleConfig::load_from_str(&format!("{OLD_CONFIG}livecoinwatch_api_key: ~\n"))
                .unwrap();
        assert_eq!(config.livecoinwatch_api_key, None);
        let config =
            OracleConfig::load_from_str(&format!("{OLD_CONFIG}livecoinwatch_api_key: abc123\n"))
                .unwrap();
        assert_eq!(config.livecoinwatch_api_key.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_default_config_round_trip() {
        let yaml = serde_yaml::to_string(&OracleConfig::default()).unwrap();
        assert!(yaml.contains("min_price_sources"));
        assert!(yaml.contains("livecoinwatch_api_key"));
        let config = OracleConfig::load_from_str(&yaml).unwrap();
        assert_eq!(config.min_price_sources, MinPriceSources::default());
    }
}
