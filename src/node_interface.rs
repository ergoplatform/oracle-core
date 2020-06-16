use reqwest::header::{HeaderValue, CONTENT_TYPE};
use crate::oracle_config::{get_node_api_key, get_node_url};
use crate::{NanoErg, BlockHeight, EpochID};


/// Registers a scan with the node and returns the `scanID`
/// To Be Implemented
pub fn register_scan(scan_name: &String, tracking_rule: &String) -> Option<String> {
    None
}

/// Get the current block height of the chain
/// To Be Implemented
pub fn current_block_height() -> BlockHeight {
    0
}



/// Gets a list of all addresses from the local unlocked node wallet
pub fn get_wallet_addresses() -> Option<Vec<String>> {
    let endpoint = get_node_url().to_owned() + "/wallet/addresses";
    println!("{:?}", endpoint);
    let client = reqwest::blocking::Client::new();
    let hapi_key = HeaderValue::from_str(&get_node_api_key()).expect("Failed to create header value from api key.");
    let mut res = client.get(&endpoint)
                .header("accept", "application/json")
                .header("api_key", hapi_key)
                .header(CONTENT_TYPE, "application/json")
                .send().ok()?;


    let mut addresses : Vec<String> = vec![];
    for segment in res.text().ok()?.split("\""){
        let seg = segment.trim();
        if seg.chars().next().unwrap() == '9' {
           addresses.push(seg.to_string()); 
        }
    }
    if addresses.len() == 0 {
        panic!("No addresses were found. Please make sure the node is running on the node-ip & node-port specified in `oracle-config.yaml` file and that your wallet is unlocked.");
    }
    Some(addresses)
}




/// Convert from Erg to nanoErg
pub fn erg_to_nanoerg(erg_amount: f64) -> u64 {
    (erg_amount * 1000000000 as f64) as u64
}


/// Convert from nanoErg to Erg
pub fn nanoerg_to_erg(nanoerg_amount: u64) -> f64 {
    (nanoerg_amount as f64) / (1000000000 as f64)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erg_conv_is_valid() {
        assert_eq!((1 as f64), nanoerg_to_erg(1000000000));
        assert_eq!((1.23 as f64), nanoerg_to_erg(1230000000));

        assert_eq!(1000000000, erg_to_nanoerg(1 as f64));
        assert_eq!(erg_to_nanoerg(3.64), 3640000000);
        assert_eq!(erg_to_nanoerg(0.64), 640000000);
        assert_eq!(erg_to_nanoerg(0.0064), 6400000);
        assert_eq!(erg_to_nanoerg(0.000000064), 64);
        assert_eq!(erg_to_nanoerg(0.000000001), 1);
    }
}