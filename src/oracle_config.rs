/// Basic functions for acquiring oracle config/node data
use yaml_rust::{Yaml, YamlLoader};

pub struct PoolParameters {
    pub live_epoch_length: u64,
    pub epoch_preparation_length: u64,
    pub buffer_length: u64,
    pub margin_of_error: f64,
}

impl PoolParameters {
    pub fn new() -> PoolParameters {
        let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
        PoolParameters::new_from_yaml_string(config)
    }

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
            .as_f64()
            .expect("No margin_of_error specified in config file.");
        PoolParameters {
            live_epoch_length: lel as u64,
            epoch_preparation_length: epl as u64,
            buffer_length: buf as u64,
            margin_of_error: moe,
        }
    }
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

/// Returns `node_api_key` from the config file
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
    fn valid_api_key_from_config() {
        assert_eq!(get_node_api_key(), "hello".to_string())
    }

    #[test]
    fn pool_parameter_parsing_works() {
        let yaml_string = "
            live_epoch_length: 20
            epoch_preparation_length: 10
            buffer_length: 4
            ";
        let config = &YamlLoader::load_from_str(yaml_string).unwrap()[0];
        let pool_params = PoolParameters::new_from_yaml_string(&config);
        assert_eq!(pool_params.live_epoch_length, 20);
        assert_eq!(pool_params.epoch_preparation_length, 10);
        assert_eq!(pool_params.buffer_length, 4);
    }
}
