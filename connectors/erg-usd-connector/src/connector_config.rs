use yaml_rust::YamlLoader;

/// Reads the `oracle-config.yaml` file
pub fn get_config_yaml() -> String {
    std::fs::read_to_string("oracle-config.yaml").expect("Failed to open oracle-config.yaml")
}

pub fn get_cmc_api_key() -> String {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["cmc_api_key"]
        .as_str()
        .expect("No cmc_api_key specified in config file.")
        .to_string()
}
