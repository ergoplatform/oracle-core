use std::convert::TryFrom;

use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::oracle_config::TokenIds;

#[derive(Clone)]
pub struct OracleContract {
    ergo_tree: ErgoTree,
    pool_nft_index: usize,
}

#[derive(Debug, From, Error)]
pub enum OracleContractError {
    #[error("oracle contract: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("oracle contract: unknown pool NFT defined in constant")]
    UnknownPoolNftId,
    #[error("oracle contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("oracle contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("oracle contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl OracleContract {
    pub fn new(
        parameters: &OracleContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, OracleContractError> {
        let ergo_tree = parameters.p2s.address().script()?.with_constant(
            parameters.pool_nft_index,
            token_ids.pool_nft_token_id.clone().into(),
        )?;
        let contract = Self::from_ergo_tree(ergo_tree, parameters, token_ids)?;
        Ok(contract)
    }

    /// Create new contract from existing ergo tree, returning error if parameters differ.
    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &OracleContractParameters,
        token_ids: &TokenIds,
    ) -> Result<Self, OracleContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        let m = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| OracleContractError::NoPoolNftId)?
            .ok_or(OracleContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>();
        match m {
            Ok(token_id) => {
                if token_id != token_ids.pool_nft_token_id {
                    return Err(OracleContractError::UnknownPoolNftId);
                }
            }
            Err(e) => {
                return Err(OracleContractError::TryExtractFrom(e));
            }
        }
        Ok(Self {
            ergo_tree,
            pool_nft_index: parameters.pool_nft_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn pool_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.pool_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    try_from = "OracleContractParametersYaml",
    into = "OracleContractParametersYaml"
)]

/// Parameters for the oracle contract
pub struct OracleContractParameters {
    pub p2s: NetworkAddress,
    pub pool_nft_index: usize,
}

/// Used to (de)serialize `OracleContractParameters` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleContractParametersYaml {
    p2s: String,
    on_mainnet: bool,
    pool_nft_index: usize,
}

impl TryFrom<OracleContractParametersYaml> for OracleContractParameters {
    type Error = AddressEncoderError;

    fn try_from(p: OracleContractParametersYaml) -> Result<Self, Self::Error> {
        let prefix = if p.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        let address = AddressEncoder::new(prefix).parse_address_from_str(&p.p2s)?;
        Ok(OracleContractParameters {
            p2s: NetworkAddress::new(prefix, &address),
            pool_nft_index: p.pool_nft_index,
        })
    }
}

impl From<OracleContractParameters> for OracleContractParametersYaml {
    fn from(val: OracleContractParameters) -> Self {
        OracleContractParametersYaml {
            p2s: val.p2s.to_base58(),
            on_mainnet: val.p2s.network() == NetworkPrefix::Mainnet,
            pool_nft_index: val.pool_nft_index,
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::{generate_token_ids, make_oracle_contract_parameters};

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_oracle_contract_parameters();
        let token_ids = generate_token_ids();
        let c = OracleContract::new(&parameters, &token_ids).unwrap();
        assert_eq!(c.pool_nft_token_id(), token_ids.pool_nft_token_id,);
    }
}
