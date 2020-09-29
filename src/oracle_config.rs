use crate::{BlockDuration, NanoErg};
use reqwest::header::HeaderValue;
use yaml_rust::{Yaml, YamlLoader};

/// Pool Parameters as defined in the `oracle-config.yaml`
pub struct PoolParameters {
    pub minimum_pool_box_value: u64,
    pub oracle_payout_price: NanoErg,
    pub live_epoch_length: BlockDuration,
    pub epoch_preparation_length: BlockDuration,
    pub buffer_length: BlockDuration,
    pub deviation_range: u64,
    pub consensus_num: u64,
    pub base_fee: u64,
}

impl PoolParameters {
    pub fn new() -> PoolParameters {
        let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
        PoolParameters::new_from_yaml_string(config)
    }

    /// Create a `PoolParameters` from a `&Yaml` string
    pub fn new_from_yaml_string(config: &Yaml) -> PoolParameters {
        let lel = config["live_epoch_length"]
            .as_i64()
            .expect("No live_epoch_length specified in config file.");
        let epl = config["epoch_preparation_length"]
            .as_i64()
            .expect("No epoch_preparation_length specified in config file.");
        let buf = config["buffer_length"]
            .as_i64()
            .expect("No buffer_length specified in config file.");
        let price = config["oracle_payout_price"]
            .as_i64()
            .expect("No oracle_payout_price specified in config file.");
        let num = config["minimum_pool_box_value"]
            .as_i64()
            .expect("No minimum_pool_box_value specified in config file.");
        let deviation_range = config["deviation_range"]
            .as_i64()
            .expect("No deviation_range specified in config file.");
        let consensus_num = config["consensus_num"]
            .as_i64()
            .expect("No consensus_num specified in config file.");
        let base_fee = config["base_fee"]
            .as_i64()
            .expect("No base_fee specified in config file.");
        PoolParameters {
            minimum_pool_box_value: num as u64,
            oracle_payout_price: price as u64,
            live_epoch_length: lel as u64,
            epoch_preparation_length: epl as u64,
            buffer_length: buf as u64,
            deviation_range: deviation_range as u64,
            consensus_num: consensus_num as u64,
            base_fee: base_fee as u64,
        }
    }
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
    std::fs::read_to_string("oracle-config.yaml").expect("Failed to open oracle-config.yaml")
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

    #[test]
    fn valid_ip_port_from_config() {
        assert_eq!(get_node_url(), "http://0.0.0.0:9053".to_string())
    }
    #[test]
    fn pool_parameter_parsing_works() {
        let yaml_string = "
            minimum_pool_box_value: 10000000
            live_epoch_length: 20
            epoch_preparation_length: 10
            buffer_length: 4
            deviation_range: 5
            oracle_payout_price: 1000000
            base_fee: 1000000
            ";
        let config = &YamlLoader::load_from_str(yaml_string).unwrap()[0];
        let pool_params = PoolParameters::new_from_yaml_string(&config);
        assert_eq!(pool_params.live_epoch_length, 20);
        assert_eq!(pool_params.epoch_preparation_length, 10);
        assert_eq!(pool_params.buffer_length, 4);
        assert_eq!(pool_params.minimum_pool_box_value, 10000000);
        assert_eq!(pool_params.deviation_range, 5);
        assert_eq!(pool_params.oracle_payout_price, 1000000);
        assert_eq!(pool_params.base_fee, 1000000);
    }
}
