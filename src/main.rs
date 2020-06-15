mod oracle_config;
mod node_interface;
mod oracle_state;

pub type NanoErg = u64;
pub type BlockHeight = u64;
pub type EpochID = String;




fn main() {
    println!("Hello, oracle pool!");

    let node_url = oracle_config::get_node_url();
    let node_api_key = oracle_config::get_node_api_key();
    let nft = oracle_config::get_oracle_pool_nft_id();
    let addresses = node_interface::get_wallet_addresses();

    println!("{:?}", addresses);
    println!("{:?}", nft);
}



