use reqwest::blocking;
use reqwest::header::{HeaderValue, CONTENT_TYPE};



/// Gets a list of all addresses from the local unlocked node wallet
pub fn get_wallet_addresses(node_ip: &String, api_key: &String) -> Vec<String> {
    let endpoint = node_ip.to_owned() + "/wallet/addresses";
    println!("{:?}", endpoint);
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&api_key).expect("Failed to create header value from api key.");
    let mut res = client.get(&endpoint)
                .header("accept", "application/json")
                .header("api_key", hapi_key)
                .header(CONTENT_TYPE, "application/json")
                .send()
                .expect("Failed to send request to local node. Please make sure it is running on the IP & Port specified in `node.ip` file.");


    let mut addresses : Vec<String> = vec![];
    for segment in res.text().expect("Failed to get addresses from wallet.").split("\""){
        let seg = segment.trim();
        if seg.chars().next().unwrap() == '9' {
           addresses.push(seg.to_string()); 
        }
    }
    if addresses.len() == 0 {
        panic!("No addresses were found. Please make sure the node is running on the node-ip & node-port specified in `oracle-config.yaml` file and that your wallet is unlocked.");
    }
    addresses
}
