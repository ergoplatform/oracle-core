use yaml_rust::{YamlLoader};


/// Reads the `oracle-config.yaml` file
pub fn get_config() -> String {
    std::fs::read_to_string("oracle-config.yaml").expect("Failed to open config.")
}

/// Returns the `ip:port` from the config file
pub fn get_node_ip_port() -> String {
    let config = &YamlLoader::load_from_str(&get_config()).unwrap()[0];
    let ip = config["node-ip"].as_str().expect("No node-ip specified in config file.");
    let port = config["node-port"].as_str().expect("No node-port specified in config file.");
    ip.to_string() + ":" + &port
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ip_port_from_config() {
        assert_eq!(get_node_ip_port(), "0.0.0.0:9053".to_string())
    }
}