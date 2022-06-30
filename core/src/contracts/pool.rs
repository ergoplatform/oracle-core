use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use thiserror::Error;

#[derive(Clone)]
pub struct PoolContract {
    ergo_tree: ErgoTree,
}

#[derive(Debug, Error)]
pub enum PoolContractError {
    #[error("pool contract: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("pool contract: failed to get refresh NFT from constants")]
    NoRefreshNftId,
}

impl PoolContract {
    // via
    // https://wallet.plutomonkey.com/p2s/?source=ewogIC8vIFRoaXMgYm94IChwb29sIGJveCkKICAvLyAgIGVwb2NoIHN0YXJ0IGhlaWdodCBpcyBzdG9yZWQgaW4gY3JlYXRpb24gSGVpZ2h0IChSMykKICAvLyAgIFI0IEN1cnJlbnQgZGF0YSBwb2ludCAoTG9uZykKICAvLyAgIFI1IEN1cnJlbnQgZXBvY2ggY291bnRlciAoSW50KQogIC8vCiAgLy8gICB0b2tlbnMoMCkgcG9vbCB0b2tlbiAoTkZUKQogIC8vICAgdG9rZW5zKDEpIHJld2FyZCB0b2tlbnMKICAvLyAgIFdoZW4gaW5pdGlhbGl6aW5nIHRoZSBib3gsIHRoZXJlIG11c3QgYmUgb25lIHJld2FyZCB0b2tlbi4gV2hlbiBjbGFpbWluZyByZXdhcmQsIG9uZSB0b2tlbiBtdXN0IGJlIGxlZnQgdW5jbGFpbWVkCiAgCiAgdmFsIG90aGVyVG9rZW5JZCA9IElOUFVUUygxKS50b2tlbnMoMCkuXzEKICB2YWwgcmVmcmVzaE5GVCA9IGZyb21CYXNlNjQoIlZHcFhibHB5TkhVM2VDRkJKVVFxUnkxTFlVNWtVbWRWYTFod01uTTFkamc9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIHVwZGF0ZU5GVCA9IGZyb21CYXNlNjQoIllsRmxWR2hYYlZweE5IUTNkeUY2SlVNcVJpMUtRRTVqVW1aVmFsaHVNbkk9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCgogIHNpZ21hUHJvcChvdGhlclRva2VuSWQgPT0gcmVmcmVzaE5GVCB8fCBvdGhlclRva2VuSWQgPT0gdXBkYXRlTkZUKQp9
    const P2S: &'static str = "PViBL5acX6PoP6BQPsYtyNzW9aPXwxpRaUkXo4nE7RkxcBbZXJECUEBQm4g3MQCb2QsQALqPkrDN9TvsKuQkChF8sZSfnH5fifgKAkXhW8ifAcAE1qA67n9mabB3Mb2R8xT2v3SN49eN8mQ8HN95";

    const REFRESH_NFT_INDEX: usize = 2;
    const UPDATE_NFT_INDEX: usize = 3;

    pub fn new() -> Self {
        let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
        let addr = encoder.parse_address_from_str(Self::P2S).unwrap();
        let ergo_tree = addr.script().unwrap();
        Self::from_ergo_tree(ergo_tree).unwrap()
    }

    pub fn from_ergo_tree(ergo_tree: ErgoTree) -> Result<Self, PoolContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        if ergo_tree
            .get_constant(Self::REFRESH_NFT_INDEX)
            .map_err(|_| PoolContractError::NoRefreshNftId)?
            .ok_or(PoolContractError::NoRefreshNftId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(PoolContractError::NoRefreshNftId);
        };

        if ergo_tree
            .get_constant(Self::UPDATE_NFT_INDEX)
            .map_err(|_| PoolContractError::NoUpdateNftId)?
            .ok_or(PoolContractError::NoUpdateNftId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(PoolContractError::NoUpdateNftId);
        };
        Ok(Self { ergo_tree })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn refresh_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(Self::REFRESH_NFT_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn with_refresh_nft_token_id(self, token_id: TokenId) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::REFRESH_NFT_INDEX, token_id.clone().into())
            .unwrap();
        Self { ergo_tree: tree }
    }

    pub fn update_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(Self::UPDATE_NFT_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn with_update_nft_token_id(self, token_id: TokenId) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::UPDATE_NFT_INDEX, token_id.clone().into())
            .unwrap();
        Self { ergo_tree: tree }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_parsing() {
        let c = PoolContract::new();
        assert_eq!(
            c.refresh_nft_token_id(),
            TokenId::from_base64("VGpXblpyNHU3eCFBJUQqRy1LYU5kUmdVa1hwMnM1djg=").unwrap()
        );
        assert_eq!(
            c.update_nft_token_id(),
            TokenId::from_base64("YlFlVGhXbVpxNHQ3dyF6JUMqRi1KQE5jUmZValhuMnI=").unwrap()
        );
    }
}
