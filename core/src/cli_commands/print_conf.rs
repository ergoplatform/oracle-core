use ergo_lib::ergotree_ir::chain::address::AddressEncoder;

use crate::oracle_config::OracleConfig;
use crate::serde::OracleConfigSerde;

pub fn print_safe_config(config: &OracleConfig) {
    let safe_config = OracleConfig {
        node_ip: "127.0.0.1".to_owned(),
        node_api_key: "hello".to_owned(),
        oracle_address: AddressEncoder::unchecked_parse_network_address_from_str(
            "3Wy3BaCjGDWE3bjjZkNo3aWaMz3cYrePMFhchcKovY9uG9vhpAuW",
        )
        .unwrap(),
        ..config.clone()
    };
    let serde_conf = OracleConfigSerde::from(safe_config);
    let s = serde_yaml::to_string(&serde_conf).unwrap();
    println!("{s}");
}
