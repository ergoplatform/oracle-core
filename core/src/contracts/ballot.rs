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
    // NOTE: slightly modified v2.0a from https://github.com/scalahub/OraclePool/blob/v2/src/main/scala/oraclepool/v2a/Contracts.scala
    // compiled via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAoYmFsbG90IGJveCk6CiAgLy8gUjQgdGhlIGdyb3VwIGVsZW1lbnQgb2YgdGhlIG93bmVyIG9mIHRoZSBiYWxsb3QgdG9rZW4gW0dyb3VwRWxlbWVudF0KICAvLyBSNSB0aGUgY3JlYXRpb24gaGVpZ2h0IG9mIHRoZSB1cGRhdGUgYm94IFtJbnRdCiAgLy8gUjYgdGhlIHZhbHVlIHZvdGVkIGZvciBbQ29sbFtCeXRlXV0KICAvLyBSNyB0aGUgcmV3YXJkIHRva2VuIGlkIFtDb2xsW0J5dGVdXQogIC8vIFI4IHRoZSByZXdhcmQgdG9rZW4gYW1vdW50IFtMb25nXQoKICB2YWwgdXBkYXRlTkZUID0gZnJvbUJhc2U2NCgiWWxGbFZHaFhiVnB4TkhRM2R5RjZKVU1xUmkxS1FFNWpVbVpWYWxodU1uST0iKSAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwgCiAgdmFsIG1pblN0b3JhZ2VSZW50ID0gMTAwMDAwMDBMICAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwgIAogIHZhbCBzZWxmUHViS2V5ID0gU0VMRi5SNFtHcm91cEVsZW1lbnRdLmdldAogIHZhbCBvdXRJbmRleCA9IGdldFZhcltJbnRdKDApLmdldAogIHZhbCBvdXRwdXQgPSBPVVRQVVRTKG91dEluZGV4KQogIAogIHZhbCBpc1NpbXBsZUNvcHkgPSBvdXRwdXQuUjRbR3JvdXBFbGVtZW50XS5pc0RlZmluZWQgICAgICAgICAgICAgICAgJiYgLy8gYmFsbG90IGJveGVzIGFyZSB0cmFuc2ZlcmFibGUgYnkgc2V0dGluZyBkaWZmZXJlbnQgdmFsdWUgaGVyZSAKICAgICAgICAgICAgICAgICAgICAgb3V0cHV0LnByb3Bvc2l0aW9uQnl0ZXMgPT0gU0VMRi5wcm9wb3NpdGlvbkJ5dGVzICYmCiAgICAgICAgICAgICAgICAgICAgIG91dHB1dC50b2tlbnMgPT0gU0VMRi50b2tlbnMgICAgICAgICAgICAgICAgICAgICAmJiAKICAgICAgICAgICAgICAgICAgICAgb3V0cHV0LnZhbHVlID49IG1pblN0b3JhZ2VSZW50IAogIAogIHZhbCB1cGRhdGUgPSBJTlBVVFMuc2l6ZSA+IDEgICAgICAgICAgICAgICAgICAgICAgICAgICAmJgogICAgICAgICAgICAgICBJTlBVVFMoMSkudG9rZW5zLnNpemUgPiAwICAgICAgICAgICAgICAgICAmJgogICAgICAgICAgICAgICBJTlBVVFMoMSkudG9rZW5zKDApLl8xID09IHVwZGF0ZU5GVCAgICAgICAmJiAvLyBjYW4gb25seSB1cGRhdGUgd2hlbiB1cGRhdGUgYm94IGlzIHRoZSAybmQgaW5wdXQKICAgICAgICAgICAgICAgb3V0cHV0LlI0W0dyb3VwRWxlbWVudF0uZ2V0ID09IHNlbGZQdWJLZXkgJiYgLy8gcHVibGljIGtleSBpcyBwcmVzZXJ2ZWQKICAgICAgICAgICAgICAgb3V0cHV0LnZhbHVlID49IFNFTEYudmFsdWUgICAgICAgICAgICAgICAgJiYgLy8gdmFsdWUgcHJlc2VydmVkIG9yIGluY3JlYXNlZAogICAgICAgICAgICAgICAhIChvdXRwdXQuUjVbQW55XS5pc0RlZmluZWQpICAgICAgICAgICAgICAgICAvLyBubyBtb3JlIHJlZ2lzdGVyczsgcHJldmVudHMgYm94IGZyb20gYmVpbmcgcmV1c2VkIGFzIGEgdmFsaWQgdm90ZSAKICAKICB2YWwgb3duZXIgPSBwcm92ZURsb2coc2VsZlB1YktleSkKICAKICAvLyB1bmxpa2UgaW4gY29sbGVjdGlvbiwgaGVyZSB3ZSBkb24ndCByZXF1aXJlIHNwZW5kZXIgdG8gYmUgb25lIG9mIHRoZSBiYWxsb3QgdG9rZW4gaG9sZGVycwogIGlzU2ltcGxlQ29weSAmJiAob3duZXIgfHwgdXBkYXRlKQp9
    const P2S: &'static str = "3whMZZ6THhGiX1avBy7KxoSNJrBEEJDhufAoWXq2qMiP5gy4ny5sVaFNrhJMybFASG7VP4DLTs2Mij6rMCQqj1D7JzMjHoguPL3W9y5g7JkWuYqrcN6AWWenaCmHa6jTueTsLmjBZnibb7L5SNJjRv1U5K9J3oazVkkmy19X2jQ3fDQ6tES8NAU1dngivpSAbuihur2tQ7ENCWeWZHkK49sUkxbWHgKRxHFFB1rT79Fs2mBp";

    const MIN_STORAGE_RENT_INDEX: usize = 0;
    const UPDATE_NFT_INDEX: usize = 6;

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
        let ergo_tree = self
            .ergo_tree
            .with_constant(Self::UPDATE_NFT_INDEX, token_id.clone().into())
            .unwrap();
        Self { ergo_tree }
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
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
