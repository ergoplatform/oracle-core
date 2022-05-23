use crate::BlockDuration;
use reqwest::header::HeaderValue;
use yaml_rust::{ScanError, ScanError, Yaml, YamlLoader};

pub const DEFAULT_CONFIG_FILE_NAME: &str = "oracle_config.yaml";

/// Node Parameters as defined in the `oracle-config.yaml`
pub struct NodeParameters {
    pub node_ip: String,
    pub node_port: String,
    pub node_api_key: String,
}

/// Pool Parameters as defined in the `oracle-config.yaml`
pub struct PoolParameters {
    pub epoch_length: BlockDuration,
    pub buffer_length: BlockDuration,
    pub max_deviation_percent: u64,
    pub min_data_points: u64,
    pub base_fee: u64,
}

pub struct OracleConfig {
    pub pool_parameters: PoolParameters,
}

impl OracleConfig {
    pub fn load() -> Result<Self, ScanError> {
        let yaml = YamlLoader::load_from_str(&get_config_yaml())?;
        let yaml = yaml[0];

        let pool_parameters = PoolParameters {
            epoch_length: yaml["epoch_length"].as_i64()? as u64,
            buffer_length: BlockDuration::from_seconds(yaml["buffer_length"].as_u64()?),
            max_deviation_percent: yaml["max_deviation_percent"].as_u64()?,
            min_data_points: yaml["min_data_points"].as_u64()?,
            base_fee: yaml["base_fee"].as_u64()?,
        };
        Ok(OracleConfig { pool_parameters })
    }
}

lazy_static! {
    static ref MAYBE_CONFIG: OracleConfig = {
        OracleConfig {
            pool_parameters: PoolParameters::load(),
        }
    };
}

impl PoolParameters {
    /// Create a `PoolParameters` from a `&Yaml` string
    pub fn new_from_yaml_string(config: &Yaml) -> PoolParameters {
        let lel = config["epoch_length"]
            .as_i64()
            .expect("No epoch_length specified in config file.");
        let buf = config["buffer_length"]
            .as_i64()
            .expect("No buffer_length specified in config file.");
        let deviation_range = config["max_deviation_percent"]
            .as_i64()
            .expect("No max_deviation_percent specified in config file.");
        let consensus_num = config["min_data_points"]
            .as_i64()
            .expect("No min_data_points specified in config file.");
        let base_fee = config["base_fee"]
            .as_i64()
            .expect("No base_fee specified in config file.");
        PoolParameters {
            epoch_length: lel as u64,
            buffer_length: buf as u64,
            max_deviation_percent: deviation_range as u64,
            min_data_points: consensus_num as u64,
            base_fee: base_fee as u64,
        }
    }
}

pub fn get_pool_deposits_contract_address() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["pool_deposit_contract_address"]
        .as_str()
        .expect("No pool_deposit_contract_address specified in config file.")
        .to_string()
}

/// Returns "core_api_port" from the config file
pub fn get_core_api_port() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["core_api_port"]
        .as_str()
        .expect("No core_api_port specified in config file.")
        .to_string()
}

/// Reads the `oracle-config.yaml` file
pub fn get_config_yaml() -> String {
    std::fs::read_to_string(DEFAULT_CONFIG_FILE_NAME).expect("Failed to open oracle-config.yaml")
}

/// Returns `http://ip:port` using `node_ip` and `node_port` from the config file
pub fn get_node_url() -> String {
    let ip = get_node_ip();
    let port = get_node_port();
    "http://".to_string() + &ip + ":" + &port
}

pub fn get_node_ip() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["node_ip"]
        .as_str()
        .expect("No node_ip specified in config file.")
        .to_string()
}

pub fn get_node_port() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["node_port"]
        .as_str()
        .expect("No node_port specified in config file.")
        .to_string()
}

/// Acquires the `node_api_key` and builds a `HeaderValue`
pub fn get_node_api_header() -> HeaderValue {
    let api_key = get_node_api_key();
    match HeaderValue::from_str(&api_key) {
        Ok(k) => k,
        _ => HeaderValue::from_static("None"),
    }
}

/// Returns the `node_api_key`
pub fn get_node_api_key() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["node_api_key"]
        .as_str()
        .expect("No node_api_key specified in config file.")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn valid_ip_port_from_config() {
    //     assert_eq!(get_node_url(), "http://0.0.0.0:9053".to_string())
    // }

    #[test]
    fn pool_parameter_parsing_works() {
        let yaml_string = "
            minimum_pool_box_value: 10000000
            epoch_length: 20
            buffer_length: 4
            max_deviation_percent: 5
            min_data_points: 4
            base_fee: 1000000
            ";
        let config = &YamlLoader::load_from_str(yaml_string).unwrap()[0];
        let pool_params = PoolParameters::new_from_yaml_string(config);
        assert_eq!(pool_params.epoch_length, 20);
        assert_eq!(pool_params.buffer_length, 4);
        assert_eq!(pool_params.max_deviation_percent, 5);
        assert_eq!(pool_params.base_fee, 1000000);
    }
}
