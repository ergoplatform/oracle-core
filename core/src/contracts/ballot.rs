use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use thiserror::Error;

#[derive(Clone)]
pub struct BallotContract {
    ergo_tree: ErgoTree,
}

#[derive(Debug, Error)]
pub enum BallotContractError {
    #[error("ballot contract: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("ballot contract: failed to get minStorageRent from constants")]
    NoMinStorageRent,
}

impl BallotContract {
    // v2.0a from https://github.com/scalahub/OraclePool/blob/v2/src/main/scala/oraclepool/v2a/Contracts.scala
    // compiled via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAoYmFsbG90IGJveCk6CiAgLy8gUjQgdGhlIGdyb3VwIGVsZW1lbnQgb2YgdGhlIG93bmVyIG9mIHRoZSBiYWxsb3QgdG9rZW4gW0dyb3VwRWxlbWVudF0KICAvLyBSNSB0aGUgY3JlYXRpb24gaGVpZ2h0IG9mIHRoZSB1cGRhdGUgYm94IFtJbnRdCiAgLy8gUjYgdGhlIHZhbHVlIHZvdGVkIGZvciBbQ29sbFtCeXRlXV0KICAvLyBSNyB0aGUgcmV3YXJkIHRva2VuIGlkIFtDb2xsW0J5dGVdXQogIC8vIFI4IHRoZSByZXdhcmQgdG9rZW4gYW1vdW50IFtMb25nXQoKICB2YWwgdXBkYXRlTkZUID0gZnJvbUJhc2U2NCgiWWxGbFZHaFhiVnB4TkhRM2R5RjZKVU1xUmkxS1FFNWpVbVpWYWxodU1uST0iKSAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwgCgogIHZhbCBtaW5TdG9yYWdlUmVudCA9IDEwMDAwMDAwTCAgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgCiAgdmFsIHNlbGZQdWJLZXkgPSBTRUxGLlI0W0dyb3VwRWxlbWVudF0uZ2V0CiAgdmFsIG90aGVyVG9rZW5JZCA9IElOUFVUUygxKS50b2tlbnMoMCkuXzEKICAKICB2YWwgb3V0SW5kZXggPSBnZXRWYXJbSW50XSgwKS5nZXQKICB2YWwgb3V0cHV0ID0gT1VUUFVUUyhvdXRJbmRleCkKICAKICB2YWwgaXNTaW1wbGVDb3B5ID0gb3V0cHV0LlI0W0dyb3VwRWxlbWVudF0uaXNEZWZpbmVkICAgICAgICAgICAgICAgICYmIC8vIGJhbGxvdCBib3hlcyBhcmUgdHJhbnNmZXJhYmxlIGJ5IHNldHRpbmcgZGlmZmVyZW50IHZhbHVlIGhlcmUgCiAgICAgICAgICAgICAgICAgICAgIG91dHB1dC5wcm9wb3NpdGlvbkJ5dGVzID09IFNFTEYucHJvcG9zaXRpb25CeXRlcyAmJgogICAgICAgICAgICAgICAgICAgICBvdXRwdXQudG9rZW5zID09IFNFTEYudG9rZW5zICAgICAgICAgICAgICAgICAgICAgJiYgCiAgICAgICAgICAgICAgICAgICAgIG91dHB1dC52YWx1ZSA+PSBtaW5TdG9yYWdlUmVudCAKICAKICB2YWwgdXBkYXRlID0gb3RoZXJUb2tlbklkID09IHVwZGF0ZU5GVCAgICAgICAgICAgICAgICAgJiYgLy8gY2FuIG9ubHkgdXBkYXRlIHdoZW4gdXBkYXRlIGJveCBpcyB0aGUgMm5kIGlucHV0CiAgICAgICAgICAgICAgIG91dHB1dC5SNFtHcm91cEVsZW1lbnRdLmdldCA9PSBzZWxmUHViS2V5ICYmIC8vIHB1YmxpYyBrZXkgaXMgcHJlc2VydmVkCiAgICAgICAgICAgICAgIG91dHB1dC52YWx1ZSA+PSBTRUxGLnZhbHVlICAgICAgICAgICAgICAgICYmIC8vIHZhbHVlIHByZXNlcnZlZCBvciBpbmNyZWFzZWQKICAgICAgICAgICAgICAgISAob3V0cHV0LlI1W0FueV0uaXNEZWZpbmVkKSAgICAgICAgICAgICAgICAgLy8gbm8gbW9yZSByZWdpc3RlcnM7IHByZXZlbnRzIGJveCBmcm9tIGJlaW5nIHJldXNlZCBhcyBhIHZhbGlkIHZvdGUgCiAgCiAgdmFsIG93bmVyID0gcHJvdmVEbG9nKHNlbGZQdWJLZXkpCiAgCiAgLy8gdW5saWtlIGluIGNvbGxlY3Rpb24sIGhlcmUgd2UgZG9uJ3QgcmVxdWlyZSBzcGVuZGVyIHRvIGJlIG9uZSBvZiB0aGUgYmFsbG90IHRva2VuIGhvbGRlcnMKICBpc1NpbXBsZUNvcHkgJiYgKG93bmVyIHx8IHVwZGF0ZSkKfQo=
    const P2S: &'static str = "2f7PHengh3DpcDM74zifyLdPqLUE6VsN9omVzn8D8EUefYuGbgUfWzM1HWmvPpw35RWaVEWXsYhEHBNyLbPxGwCj18neiq66mhrxgL7YMeGUTjcvh1cpmFtVjBkrtVibZdWk8hWPyjFic8i2hPYaC22xsgV7wrf96y9MWBLPvRK6BnAb4SXhJm3W7sEmzZGutuQkycbHj7kBD9";
    const MIN_STORAGE_RENT_INDEX: usize = 0;
    const UPDATE_NFT_INDEX: usize = 3;

    pub fn new() -> Self {
        let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
        let addr = encoder.parse_address_from_str(Self::P2S).unwrap();
        let ergo_tree = addr.script().unwrap();
        Self::from_ergo_tree(ergo_tree).unwrap()
    }

    pub fn from_ergo_tree(ergo_tree: ErgoTree) -> Result<Self, BallotContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        if ergo_tree
            .get_constant(Self::MIN_STORAGE_RENT_INDEX)
            .map_err(|_| BallotContractError::NoMinStorageRent)?
            .ok_or(BallotContractError::NoMinStorageRent)?
            .try_extract_into::<i64>()
            .is_err()
        {
            return Err(BallotContractError::NoMinStorageRent);
        };

        if ergo_tree
            .get_constant(Self::UPDATE_NFT_INDEX)
            .map_err(|_| BallotContractError::NoUpdateNftId)?
            .ok_or(BallotContractError::NoUpdateNftId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(BallotContractError::NoUpdateNftId);
        };
        Ok(Self { ergo_tree })
    }

    pub fn min_storage_rent(&self) -> u64 {
        self.ergo_tree
            .get_constant(Self::MIN_STORAGE_RENT_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }

    pub fn with_min_storage_rent(self, min_storage_rent: u64) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(
                Self::MIN_STORAGE_RENT_INDEX,
                (min_storage_rent as i64).into(),
            )
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
        let c = BallotContract::new();
        assert_eq!(
            c.update_nft_token_id(),
            TokenId::from_base64("YlFlVGhXbVpxNHQ3dyF6JUMqRi1KQE5jUmZValhuMnI=").unwrap()
        );
        assert_eq!(c.min_storage_rent(), 10000000);
    }
}
