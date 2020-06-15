use yaml_rust::{YamlLoader};


/// Reads the `oracle-config.yaml` file
fn get_config_yaml() -> String {
    std::fs::read_to_string("oracle-config.yaml").expect("Failed to open config.")
}

/// Returns `http://ip:port` using node-ip and node-port from the config file
pub fn get_node_url() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    let ip = config["node-ip"].as_str().expect("No node-ip specified in config file.");
    let port = config["node-port"].as_str().expect("No node-port specified in config file.");
    "http://".to_string() + ip + ":" + &port
}

pub fn get_node_api_key() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["node-api-key"].as_str().expect("No node-api-key specified in config file.").to_string()
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
}