use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
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
    url: String,
    port: String,
}

#[derive(Serialize, Deserialize)]
pub struct OracleInfo {
    pub oracle_address: String,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct OracleStatus {
    pub waiting_for_datapoint_submit: String,
    pub latest_datapoint: String,
    pub latest_datapoint_epoch: String,
    pub latest_datapoint_creation_height: String,
}

#[derive(Serialize, Deserialize)]
pub struct PoolStatus {
    pub funded_percentage: String,
    pub current_pool_stage: String,
}

impl OracleCore {
    pub fn get_block_height() -> u64 {
        1
    }

    pub fn submit_datapoint() -> Result<String> {
        Ok("Soon".to_string())
    }
}

/// Sends a GET request to the Ergo node
fn send_get_req(core_url: &str) -> Result<Response> {
    let client = reqwest::blocking::Client::new().get(core_url);
    client.send().map_err(|_| ConnectorError::CoreUnreachable)
}

/// Sends a POST request to the Ergo node
fn send_post_req(core_url: &str, body: String) -> Result<Response> {
    let client = reqwest::blocking::Client::new().post(core_url);
    client
        .body(body)
        .send()
        .map_err(|_| ConnectorError::CoreUnreachable)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
