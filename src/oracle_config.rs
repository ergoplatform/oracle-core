use crate::{BlockDuration, NanoErg};
use reqwest::header::HeaderValue;
/// Basic functions for acquiring oracle config/node data
use yaml_rust::{Yaml, YamlLoader};

pub struct PoolParameters {
    pub number_of_oracles: u64,
    pub oracle_payout_price: NanoErg,
    pub live_epoch_length: BlockDuration,
    pub epoch_preparation_length: BlockDuration,
    pub buffer_length: BlockDuration,
    pub margin_of_error: u64,
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
        let moe = config["margin_of_error"]
            .as_i64()
            .expect("No margin_of_error specified in config file.");
        let price = config["oracle_payout_price"]
            .as_i64()
            .expect("No oracle_payout_price specified in config file.");
        let num = config["number_of_oracles"]
            .as_i64()
            .expect("No number_of_oracles specified in config file.");
        let base_fee = config["base_fee"]
            .as_i64()
            .expect("No base_fee specified in config file.");
        PoolParameters {
            number_of_oracles: num as u64,
            oracle_payout_price: price as u64,
            live_epoch_length: lel as u64,
            epoch_preparation_length: epl as u64,
            buffer_length: buf as u64,
            margin_of_error: moe as u64,
            base_fee: base_fee as u64,
        }
    }

    /// Calculates the maximum total payout that the oracle pool will require
    /// in order to payout all of the oracles + the collector.
    pub fn max_pool_payout(&self) -> NanoErg {
        self.oracle_payout_price * (self.number_of_oracles + 1)
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

/// Reads the `oracle_config.yaml` file
pub fn get_config_yaml() -> String {
    std::fs::read_to_string("oracle-config.yaml").expect("Failed to open oracle_config.yaml")
}

/// Returns `http://ip:port` using `node_ip` and `node_port` from the config file
pub fn get_node_url() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    let ip = config["node_ip"]
        .as_str()
        .expect("No node_ip specified in config file.");
    let port = config["node_port"]
        .as_str()
        .expect("No node_port specified in config file.");
    "http://".to_string() + ip + ":" + &port
}

/// Acquires the `node_api_key` and builds a `HeaderValue`
pub fn get_node_api_header() -> HeaderValue {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    let api_key = config["node_api_key"]
        .as_str()
        .expect("No node_api_key specified in config file.")
        .to_string();

    match HeaderValue::from_str(&api_key) {
        Ok(k) => k,
        _ => HeaderValue::from_static("None"),
    }
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
            number_of_oracles: 4
            live_epoch_length: 20
            epoch_preparation_length: 10
            buffer_length: 4
            margin_of_error: 0.01
            oracle_payout_price: 1000000
            base_fee: 1000000
            ";
        let config = &YamlLoader::load_from_str(yaml_string).unwrap()[0];
        let pool_params = PoolParameters::new_from_yaml_string(&config);
        assert_eq!(pool_params.live_epoch_length, 20);
        assert_eq!(pool_params.epoch_preparation_length, 10);
        assert_eq!(pool_params.buffer_length, 4);
        assert_eq!(pool_params.number_of_oracles, 4);
        assert_eq!(pool_params.margin_of_error, 10);
        assert_eq!(pool_params.oracle_payout_price, 1000000);
        assert_eq!(pool_params.base_fee, 1000000);
    }
}
