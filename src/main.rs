mod oracle_config;
mod node_interface;


fn main() {
    println!("Hello, oracle pool!");

    let node_url = oracle_config::get_node_url();
    let node_api_key = oracle_config::get_node_api_key();
    let addresses = node_interface::get_wallet_addresses(&node_url, &node_api_key);

    println!("{:?}", addresses);
}



