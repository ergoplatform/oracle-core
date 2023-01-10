use std::time::Duration;

use derive_more::From;
use ergo_lib::chain::transaction::Transaction;
use ergo_lib::chain::transaction::TxId;
use reqwest::blocking::RequestBuilder;
use reqwest::blocking::Response;
use reqwest::header::CONTENT_TYPE;
use thiserror::Error;
use url::ParseError;

#[derive(Debug, From, Error)]
pub enum ExplorerApiError {
    #[error("reqwest error: {0}")]
    RequestError(reqwest::Error),
    #[error("serde error: {0}")]
    SerdeError(serde_json::Error),
    #[error("invalid explorer url: {0}")]
    InvalidExplorerUrl(ParseError),
}

pub struct ExplorerApi {
    pub url: url::Url,
}

impl ExplorerApi {
    pub fn new(url: &str) -> Result<Self, ExplorerApiError> {
        Ok(Self {
            url: url::Url::parse(url)?,
        })
    }

    /// Sets required headers for a request
    fn set_req_headers(&self, rb: RequestBuilder) -> RequestBuilder {
        rb.header("accept", "application/json")
            .header(CONTENT_TYPE, "application/json")
    }

    /// Sends a GET request to the Ergo node
    fn send_get_req(&self, endpoint: &str) -> Result<Response, ExplorerApiError> {
        let url = self.url.join(endpoint)?;
        let client = reqwest::blocking::Client::new().get(url);
        Ok(self.set_req_headers(client).send()?)
    }

    /// GET /transactions/{id}
    pub fn get_transaction(&self, tx_id: TxId) -> Result<Transaction, ExplorerApiError> {
        let endpoint = "/transactions/".to_owned() + &tx_id.to_string();
        let response = self.send_get_req(&endpoint)?;
        let text = response.text()?;
        Ok(serde_json::from_str(&text)?)
    }
}

pub fn wait_for_tx_confirmation(tx_id: TxId) {
    let timeout = Duration::from_secs(3600);
    let explorer_api = ExplorerApi::new("https://api.ergoplatform.com/api/v1/").unwrap();
    let start_time = std::time::Instant::now();
    println!("Waiting for block confirmation from ExplorerApi ...");
    loop {
        if let Ok(tx) = explorer_api.get_transaction(tx_id) {
            log::info!("Transaction found: {}", tx.id());
            break;
        }
        if start_time.elapsed() > timeout {
            println!("Timeout waiting for transaction");
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
