use crate::spec_token::TokenIdKind;
use crate::NodeApi;
use derive_more::From;
use derive_more::Into;
use ergo_node_interface::ScanId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

use super::NodeScan;
use super::NodeScanId;
use super::ScanError;
use super::ScanGetBoxes;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, From, Into)]
#[serde(try_from = "String", into = "String")]
pub struct GenericTokenScan<T: TokenIdKind + std::clone::Clone> {
    id: ScanId,
    fantom: std::marker::PhantomData<T>,
}

impl<T: TokenIdKind + Clone> GenericTokenScan<T> {
    pub fn new(id: ScanId) -> Self {
        Self {
            id,
            fantom: std::marker::PhantomData,
        }
    }

    pub fn register(node_api: &NodeApi, token_id: &T) -> Result<Self, ScanError> {
        let scan_name = format!("token scan for  {}", String::from(token_id.token_id()));
        let id = node_api.register_scan(scan_name, Self::tracking_rule(token_id))?;
        Ok(GenericTokenScan::<T> {
            id,
            fantom: std::marker::PhantomData,
        })
    }

    pub fn tracking_rule(token_id: &T) -> serde_json::Value {
        json!({
        "predicate": "and",
        "args":
            [
                {
                    "predicate": "containsAsset",
                    "assetId": token_id.token_id(),
                }
            ]
          })
    }
}

impl<T: TokenIdKind + Clone> TryFrom<String> for GenericTokenScan<T> {
    type Error = ScanError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let id = value.parse::<u64>().unwrap().into();
        Ok(GenericTokenScan {
            id,
            fantom: std::marker::PhantomData,
        })
    }
}

impl<T: TokenIdKind + Clone> From<GenericTokenScan<T>> for String {
    fn from(scan: GenericTokenScan<T>) -> Self {
        scan.id.to_string()
    }
}

impl<T: TokenIdKind + Clone> NodeScanId for GenericTokenScan<T> {
    fn scan_id(&self) -> ScanId {
        self.id
    }
}

impl<T: TokenIdKind + Clone> NodeScan for GenericTokenScan<T> {
    #[allow(clippy::todo)]
    fn scan_name(&self) -> &'static str {
        todo!()
    }
}

impl<T: TokenIdKind + Clone> ScanGetBoxes for GenericTokenScan<T> {}
