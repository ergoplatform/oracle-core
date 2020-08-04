#[macro_use]
extern crate json;
use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use thiserror::Error;
use yaml_rust::{Yaml, YamlLoader};

pub type Result<T> = std::result::Result<T, ConnectorError>;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("The configured oracle core is unreachable. Please ensure your config is correctly filled out and the core is running.")]
    CoreUnreachable,
    #[error("Failed reading response from core.")]
    FailedParsingCoreResponse,
    #[error("Failed opening the local `oracle-config.yaml` file.")]
    FailedOpeningOracleConfigFile,
    #[error("Datapoint Error: {0}")]
    FailedSubmittingDatapoint(String),
}

/// The base struct for interfacing with the Oracle Core.
/// All methods are implemented on this struct.
pub struct OracleCore {
    pub ip: String,
    pub port: String,
}

/// Info about the local Oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleInfo {
    pub oracle_address: String,
}

/// Info about the Oracle Pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub live_epoch_address: String,
    pub epoch_prep_address: String,
    pub pool_deposits_address: String,
    pub datapoint_address: String,
    pub oracle_payout_price: u64,
    pub live_epoch_length: u64,
    pub epoch_prep_length: u64,
    pub margin_of_error: f64,
    pub number_of_oracles: u64,
    pub oracle_pool_nft_id: String,
    pub oracle_pool_participant_token_id: String,
}

/// Info about the Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_url: String,
}

/// Status of the local Oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleStatus {
    pub waiting_for_datapoint_submit: bool,
    pub latest_datapoint: u64,
    pub latest_datapoint_epoch: String,
    pub latest_datapoint_creation_height: u64,
}

/// Status of the Oracle Pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    pub funded_percentage: u64,
    pub current_pool_stage: String,
}

impl OracleCore {
    /// Create a new `OracleCore` struct for use with your Connector
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

    /// Submit a u64 Datapoint to the Oracle Core
    pub fn submit_datapoint(&self, datapoint: u64) -> Result<String> {
        let datapoint_json = object! { datapoint: datapoint};
        let resp_text = self.send_post_req("/submitDatapoint", datapoint_json.dump())?;
        // Add error checking here by parsing the json.
        if let Ok(resp_json) = json::parse(&resp_text) {
            let tx_id = resp_json["tx_id"].clone();

            // If there no tx_id/there is an error
            if tx_id.is_empty() {
                let error = resp_json["error"].clone();
                return Err(ConnectorError::FailedSubmittingDatapoint(error.to_string()));
            } else {
                return Ok(tx_id.to_string());
            }
        } else {
            return Err(ConnectorError::FailedParsingCoreResponse);
        }
    }
    /// Get information about the local Oracle
    pub fn oracle_info(&self) -> Result<OracleInfo> {
        let resp_text = self.send_get_req("/oracleInfo")?;
        from_str(&resp_text).map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Get information about the Oracle Pool
    pub fn pool_info(&self) -> Result<PoolInfo> {
        let resp_text = self.send_get_req("/poolInfo")?;
        from_str(&resp_text).map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Get node info
    pub fn node_info(&self) -> Result<NodeInfo> {
        let resp_text = self.send_get_req("/nodeInfo")?;
        from_str(&resp_text).map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Get the current local Oracle Status
    pub fn oracle_status(&self) -> Result<OracleStatus> {
        let resp_text = self.send_get_req("/oracleStatus")?;
        from_str(&resp_text).map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Get the current Oracle Pool Status
    pub fn pool_status(&self) -> Result<PoolStatus> {
        let resp_text = self.send_get_req("/poolStatus")?;
        from_str(&resp_text).map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Get the current block height
    pub fn current_block_height(&self) -> Result<u64> {
        let resp_text = self.send_get_req("/blockHeight")?;
        resp_text
            .parse()
            .map_err(|_| ConnectorError::FailedParsingCoreResponse)
    }

    /// Sends a GET request to the Oracle Core and converts response to text with extra quotes removed
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

        // Check if returned response has quotes around it which need to be removed
        if &text[0..1] == "\"" {
            // Remove quotes before returning
            return Ok(text[1..(text.len() - 1)].to_string());
        }
        Ok(text)
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

/// Reads the local `oracle_config.yaml` file
fn get_config_yaml_string() -> Result<String> {
    std::fs::read_to_string("oracle-config.yaml")
        .map_err(|_| ConnectorError::FailedOpeningOracleConfigFile)
}

/// Returns "core_api_port" from the local config file
pub fn get_core_api_port() -> Result<String> {
    let config_string = get_config_yaml_string()?;
    let config = &YamlLoader::load_from_str(&config_string)
        .map_err(|_| ConnectorError::FailedOpeningOracleConfigFile)?[0];
    if let Some(s) = config["core_api_port"].as_str() {
        Ok(s.to_string())
    } else {
        Err(ConnectorError::FailedOpeningOracleConfigFile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static IP: &str = "0.0.0.0";
    static PORT: &str = "9090";

    #[test]
    fn test_core_api_get() {
        if let Err(e) = get_core_api_port() {
            println!("{:?}", e);
            panic!("Test Oracle Info Failed.")
        }
    }

    #[test]
    fn test_current_block_height() {
        let oc = OracleCore::new(IP, PORT);
        if let Err(e) = oc.current_block_height() {
            println!("{:?}", e);
            panic!("Test Oracle Info Failed.")
        }
    }

    #[test]
    fn test_oracle_info() {
        let oc = OracleCore::new(IP, PORT);
        if let Err(e) = oc.oracle_info() {
            println!("{:?}", e);
            panic!("Test Oracle Info Failed.")
        }
    }

    #[test]
    fn test_pool_info() {
        let oc = OracleCore::new(IP, PORT);
        if let Err(e) = oc.pool_info() {
            println!("{:?}", e);
            panic!("Test Pool Info Failed.")
        }
    }

    #[test]
    fn test_node_info() {
        let oc = OracleCore::new(IP, PORT);
        if let Err(e) = oc.node_info() {
            println!("{:?}", e);
            panic!("Test Node Info Failed.")
        }
    }

    #[test]
    fn test_oracle_status() {
        let oc = OracleCore::new(IP, PORT);
        if let Err(e) = oc.oracle_status() {
            println!("{:?}", e);
            panic!("Test Oracle Status Failed.")
        }
    }

    #[test]
    fn test_pool_status() {
        let oc = OracleCore::new(IP, PORT);
        if let Err(e) = oc.pool_status() {
            println!("{:?}", e);
            panic!("Test Pool Status Failed.")
        }
    }
}
