use crate::spec_token::TokenIdKind;
use crate::NodeApi;
use derive_more::From;
use derive_more::Into;
use ergo_node_interface::ScanId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

use crate::spec_token::OracleTokenId;

use super::NodeScan;
use super::NodeScanId;
use super::ScanError;
use super::ScanGetBoxes;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, From, Into)]
#[serde(try_from = "String", into = "String")]
pub struct OracleTokenScan(ScanId);

impl OracleTokenScan {
    pub fn tracking_rule(oracle_token_id: &OracleTokenId) -> serde_json::Value {
        json!({
        "predicate": "and",
        "args":
            [
                {
                    "predicate": "containsAsset",
                    "assetId": oracle_token_id,
                }
            ]
          })
    }

    pub fn register(
        node_api: &NodeApi,
        oracle_token_id: &OracleTokenId,
    ) -> Result<Self, ScanError> {
        let scan_name = format!(
            "oracle token scan {}",
            String::from(oracle_token_id.token_id())
        );
        let id = node_api.register_scan(scan_name, Self::tracking_rule(oracle_token_id))?;
        Ok(OracleTokenScan(id))
    }
}

impl TryFrom<String> for OracleTokenScan {
    type Error = ScanError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let id = value.parse::<u64>().unwrap().into();
        Ok(OracleTokenScan(id))
    }
}

impl From<OracleTokenScan> for String {
    fn from(scan: OracleTokenScan) -> Self {
        scan.0.to_string()
    }
}

impl NodeScanId for OracleTokenScan {
    fn scan_id(&self) -> ScanId {
        self.0
    }
}

impl NodeScan for OracleTokenScan {
    #[allow(clippy::todo)]
    fn scan_name(&self) -> &'static str {
        todo!()
    }
}

impl ScanGetBoxes for OracleTokenScan {}
