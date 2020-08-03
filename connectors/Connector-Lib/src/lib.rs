use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ConnectorError>;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("The configured oracle core is unreachable. Please ensure your config is correctly filled out and the core is running.")]
    CoreUnreachable,
    #[error("Failed reading response from core.")]
    FailedParsingCoreResponse,
}

pub struct OracleCore {
    pub ip: String,
    pub port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleInfo {
    pub oracle_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub live_epoch_address: String,
    pub epoch_prep_address: String,
    pub pool_deposits_address: String,
    pub datapoint_address: String,
    pub oracle_payout_price: String,
    pub live_epoch_length: String,
    pub epoch_prep_length: String,
    pub margin_of_error: String,
    pub number_of_oracles: String,
    pub oracle_pool_nft_id: String,
    pub oracle_pool_participant_token_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleStatus {
    pub waiting_for_datapoint_submit: String,
    pub latest_datapoint: String,
    pub latest_datapoint_epoch: String,
    pub latest_datapoint_creation_height: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    pub funded_percentage: u64,
    pub current_pool_stage: String,
}

impl OracleCore {
    pub fn new(ip: &str, port: &str) -> OracleCore {
        OracleCore {
            ip: ip.to_string(),
            port: port.to_string(),
        }
    }

    /// Returns the url of the Oracle Core
    pub fn oracle_core_url(&self) -> String {
        "http://".to_string() + &self.ip + ":" + &self.port
    }

    pub fn get_pool_status(&self) -> Result<PoolStatus> {
        let resp_text = self.send_get_req("/poolStatus")?;
        println!("{}", resp_text);
        let fs = from_str(&resp_text);
        println!("fs: {:?}", fs);
        fs.map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Get the current block height
    pub fn get_block_height(&self) -> Result<u64> {
        Ok(1)
    }

    /// Submit a u64 Datapoint to the Oracle Core
    pub fn submit_datapoint(&self, datapoint: u64) -> Result<String> {
        Ok("Soon".to_string())
    }

    /// Sends a GET request to the Oracle Core and converts response to text
    fn send_get_req(&self, endpoint: &str) -> Result<String> {
        let url = self.oracle_core_url().to_owned() + endpoint;
        let resp = reqwest::blocking::Client::new()
            .get(&url)
            .send()
            .map_err(|_| ConnectorError::CoreUnreachable)?;
        let text: String = resp
            .text()
            .map(|s| s.chars().filter(|&c| c != '\\').collect())
            .map_err(|_| ConnectorError::FailedParsingCoreResponse)?;
        Ok(text[1..(text.len() - 1)].to_string())
    }

    /// Sends a POST request to the Oracle Core and converts response to text
    fn send_post_req(&self, endpoint: &str, body: String) -> Result<String> {
        let url = self.oracle_core_url().to_owned() + endpoint;
        let resp = reqwest::blocking::Client::new()
            .post(&url)
            .body(body)
            .send()
            .map_err(|_| ConnectorError::CoreUnreachable)?;
        let text: String = resp
            .text()
            .map(|s| s.chars().filter(|&c| c != '\\').collect())
            .map_err(|_| ConnectorError::FailedParsingCoreResponse)?;
        Ok(text[1..(text.len() - 1)].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_status() {
        let oc = OracleCore::new("0.0.0.0", "9090");
        if let Err(_) = oc.get_pool_status() {
            panic!("Test Pool Status Failed.")
        }
    }

    #[test]
    fn test_pool_status() {
        let oc = OracleCore::new("0.0.0.0", "9090");
        if let Err(_) = oc.get_pool_status() {
            panic!("Test Pool Status Failed.")
        }
    }
}
