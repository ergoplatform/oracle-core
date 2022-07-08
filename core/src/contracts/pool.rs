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

#[derive(Clone)]
pub struct PoolContract {
    ergo_tree: ErgoTree,
    refresh_nft_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, From, Error)]
pub enum PoolContractError {
    #[error("pool contract: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("pool contract: failed to get refresh NFT from constants")]
    NoRefreshNftId,
    #[error("pool contract: unknown refresh NFT in box")]
    UnknownRefreshNftId,
    #[error("pool contract: unknown update NFT in box")]
    UnknownUpdateNftId,
    #[error("pool contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("pool contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("pool contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl PoolContract {
    pub fn new(parameters: &PoolContractParameters) -> Result<Self, PoolContractError> {
        let ergo_tree = parameters
            .p2s
            .address()
            .script()?
            .with_constant(
                parameters.refresh_nft_index,
                parameters.refresh_nft_token_id.clone().into(),
            )?
            .with_constant(
                parameters.update_nft_index,
                parameters.update_nft_token_id.clone().into(),
            )?;
        let contract = Self::from_ergo_tree(ergo_tree, parameters)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &PoolContractParameters,
    ) -> Result<Self, PoolContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let token_id = ergo_tree
            .get_constant(parameters.refresh_nft_index)
            .map_err(|_| PoolContractError::NoRefreshNftId)?
            .ok_or(PoolContractError::NoRefreshNftId)?
            .try_extract_into::<TokenId>();
        match token_id {
            Ok(token_id) => {
                if token_id != parameters.refresh_nft_token_id {
                    return Err(PoolContractError::UnknownRefreshNftId);
                }
            }
            Err(e) => {
                return Err(PoolContractError::TryExtractFrom(e));
            }
        };

        let token_id = ergo_tree
            .get_constant(parameters.update_nft_index)
            .map_err(|_| PoolContractError::NoUpdateNftId)?
            .ok_or(PoolContractError::NoUpdateNftId)?
            .try_extract_into::<TokenId>();
        match token_id {
            Ok(token_id) => {
                if token_id != parameters.update_nft_token_id {
                    return Err(PoolContractError::UnknownUpdateNftId);
                }
            }
            Err(e) => {
                return Err(PoolContractError::TryExtractFrom(e));
            }
        };
        Ok(Self {
            ergo_tree,
            refresh_nft_index: parameters.refresh_nft_index,
            update_nft_index: parameters.update_nft_index,
        })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn refresh_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.refresh_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn update_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.update_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    try_from = "PoolContractParametersYaml",
    into = "PoolContractParametersYaml"
)]
/// Parameters for the pool contract
pub struct PoolContractParameters {
    pub p2s: NetworkAddress,
    pub refresh_nft_index: usize,
    pub refresh_nft_token_id: TokenId,
    pub update_nft_index: usize,
    pub update_nft_token_id: TokenId,
}

/// Used to (de)serialize `PoolContractParameters` instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoolContractParametersYaml {
    p2s: String,
    on_mainnet: bool,
    pub refresh_nft_index: usize,
    pub refresh_nft_token_id: TokenId,
    pub update_nft_index: usize,
    pub update_nft_token_id: TokenId,
}

impl TryFrom<PoolContractParametersYaml> for PoolContractParameters {
    type Error = AddressEncoderError;

    fn try_from(p: PoolContractParametersYaml) -> Result<Self, Self::Error> {
        let prefix = if p.on_mainnet {
            NetworkPrefix::Mainnet
        } else {
            NetworkPrefix::Testnet
        };
        let address = AddressEncoder::new(prefix).parse_address_from_str(&p.p2s)?;
        Ok(PoolContractParameters {
            p2s: NetworkAddress::new(prefix, &address),
            refresh_nft_index: p.refresh_nft_index,
            refresh_nft_token_id: p.refresh_nft_token_id,
            update_nft_index: p.update_nft_index,
            update_nft_token_id: p.update_nft_token_id,
        })
    }
}

impl From<PoolContractParameters> for PoolContractParametersYaml {
    fn from(val: PoolContractParameters) -> Self {
        PoolContractParametersYaml {
            p2s: val.p2s.to_base58(),
            on_mainnet: val.p2s.network() == NetworkPrefix::Mainnet,
            refresh_nft_index: val.refresh_nft_index,
            refresh_nft_token_id: val.refresh_nft_token_id.clone(),
            update_nft_index: val.update_nft_index,
            update_nft_token_id: val.update_nft_token_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::make_pool_contract_parameters;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_pool_contract_parameters();
        let refresh_nft_token_id = parameters.refresh_nft_token_id.clone();
        let update_nft_token_id = parameters.update_nft_token_id.clone();
        let c = PoolContract::new(&parameters).unwrap();
        assert_eq!(c.refresh_nft_token_id(), refresh_nft_token_id,);
        assert_eq!(c.update_nft_token_id(), update_nft_token_id,);
    }
}
