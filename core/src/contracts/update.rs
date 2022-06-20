use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::{Literal, TryExtractInto};
use ergo_lib::ergotree_ir::types::stype::SType;

use thiserror::Error;

#[derive(Clone)]
pub struct UpdateContract {
    ergo_tree: ErgoTree,
}

#[derive(Debug, Error)]
pub enum UpdateContractError {
    #[error("update contract: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("update contract: failed to get ballot token id from constants")]
    NoBallotTokenId,
    #[error("update: failed to get minimum votes (must be SInt)")]
    MinVotesError,
}

impl UpdateContract {
    // from https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAodXBkYXRlIGJveCk6CiAgLy8gUmVnaXN0ZXJzIGVtcHR5IAogIC8vIAogIC8vIGJhbGxvdCBib3hlcyAoSW5wdXRzKQogIC8vIFI0IHRoZSBwdWIga2V5IG9mIHZvdGVyIFtHcm91cEVsZW1lbnRdIChub3QgdXNlZCBoZXJlKQogIC8vIFI1IHRoZSBjcmVhdGlvbiBoZWlnaHQgb2YgdGhpcyBib3ggW0ludF0KICAvLyBSNiB0aGUgdmFsdWUgdm90ZWQgZm9yIFtDb2xsW0J5dGVdXSAoaGFzaCBvZiB0aGUgbmV3IHBvb2wgYm94IHNjcmlwdCkKCiAgdmFsIHBvb2xORlQgPSBmcm9tQmFzZTY0KCJSeXRMWWxCbFUyaFdiVmx4TTNRMmR6bDZKRU1tUmlsS1FFMWpVV1pVYWxjPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKCiAgdmFsIGJhbGxvdFRva2VuSWQgPSBmcm9tQmFzZTY0KCJQMFFvUnkxTFlWQmtVMmRXYTFsd00zTTJkamw1SkVJbVJTbElRRTFpVVdVPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKCiAgdmFsIG1pblZvdGVzID0gNiAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwKICAKICB2YWwgcG9vbEluID0gSU5QVVRTKDApIC8vIHBvb2wgYm94IGlzIDFzdCBpbnB1dAogIHZhbCBwb29sT3V0ID0gT1VUUFVUUygwKSAvLyBjb3B5IG9mIHBvb2wgYm94IGlzIHRoZSAxc3Qgb3V0cHV0CgogIHZhbCB1cGRhdGVCb3hPdXQgPSBPVVRQVVRTKDEpIC8vIGNvcHkgb2YgdGhpcyBib3ggaXMgdGhlIDJuZCBvdXRwdXQKCiAgLy8gY29tcHV0ZSB0aGUgaGFzaCBvZiB0aGUgcG9vbCBvdXRwdXQgYm94LiBUaGlzIHNob3VsZCBiZSB0aGUgdmFsdWUgdm90ZWQgZm9yCiAgdmFsIHBvb2xPdXRIYXNoID0gYmxha2UyYjI1Nihwb29sT3V0LnByb3Bvc2l0aW9uQnl0ZXMpCiAgCiAgdmFsIHZhbGlkUG9vbEluID0gcG9vbEluLnRva2VucygwKS5fMSA9PSBwb29sTkZUCiAgCiAgdmFsIHZhbGlkUG9vbE91dCA9IHBvb2xJbi5wcm9wb3NpdGlvbkJ5dGVzICE9IHBvb2xPdXQucHJvcG9zaXRpb25CeXRlcyAgJiYgLy8gc2NyaXB0IHNob3VsZCBub3QgYmUgcHJlc2VydmVkCiAgICAgICAgICAgICAgICAgICAgIHBvb2xJbi50b2tlbnMgPT0gcG9vbE91dC50b2tlbnMgICAgICAgICAgICAgICAgICAgICAgJiYgLy8gdG9rZW5zIHByZXNlcnZlZAogICAgICAgICAgICAgICAgICAgICBwb29sSW4uY3JlYXRpb25JbmZvLl8xID09IHBvb2xPdXQuY3JlYXRpb25JbmZvLl8xICAgICYmIC8vIGNyZWF0aW9uIGhlaWdodCBwcmVzZXJ2ZWQKICAgICAgICAgICAgICAgICAgICAgcG9vbEluLnZhbHVlID09IHBvb2xPdXQudmFsdWUgICAgICAgICAgICAgICAgICAgICAgICAmJiAvLyB2YWx1ZSBwcmVzZXJ2ZWQgCiAgICAgICAgICAgICAgICAgICAgIHBvb2xJbi5SNFtMb25nXSA9PSBwb29sT3V0LlI0W0xvbmddICAgICAgICAgICAgICAgICAgJiYgLy8gcmF0ZSBwcmVzZXJ2ZWQgIAogICAgICAgICAgICAgICAgICAgICBwb29sSW4uUjVbSW50XSA9PSBwb29sT3V0LlI1W0ludF0gICAgICAgICAgICAgICAgICAgICYmIC8vIGNvdW50ZXIgcHJlc2VydmVkCiAgICAgICAgICAgICAgICAgICAgICEgKHBvb2xPdXQuUjZbQW55XS5pc0RlZmluZWQpCgogIAogIHZhbCB2YWxpZFVwZGF0ZU91dCA9IHVwZGF0ZUJveE91dC50b2tlbnMgPT0gU0VMRi50b2tlbnMgICAgICAgICAgICAgICAgICAgICAmJgogICAgICAgICAgICAgICAgICAgICAgIHVwZGF0ZUJveE91dC5wcm9wb3NpdGlvbkJ5dGVzID09IFNFTEYucHJvcG9zaXRpb25CeXRlcyAmJgogICAgICAgICAgICAgICAgICAgICAgIHVwZGF0ZUJveE91dC52YWx1ZSA+PSBTRUxGLnZhbHVlICAgICAgICAgICAgICAgICAgICAgICAmJgogICAgICAgICAgICAgICAgICAgICAgIHVwZGF0ZUJveE91dC5jcmVhdGlvbkluZm8uXzEgPiBTRUxGLmNyZWF0aW9uSW5mby5fMSAgICAmJgogICAgICAgICAgICAgICAgICAgICAgICEgKHVwZGF0ZUJveE91dC5SNFtBbnldLmlzRGVmaW5lZCkgCgogIGRlZiBpc1ZhbGlkQmFsbG90KGI6Qm94KSA9IGlmIChiLnRva2Vucy5zaXplID4gMCkgewogICAgYi50b2tlbnMoMCkuXzEgPT0gYmFsbG90VG9rZW5JZCAgICAgICAmJgogICAgYi5SNVtJbnRdLmdldCA9PSBTRUxGLmNyZWF0aW9uSW5mby5fMSAmJiAvLyBlbnN1cmUgdm90ZSBjb3JyZXNwb25kcyB0byB0aGlzIGJveCBieSBjaGVja2luZyBjcmVhdGlvbiBoZWlnaHQKICAgIGIuUjZbQ29sbFtCeXRlXV0uZ2V0ID09IHBvb2xPdXRIYXNoICAgICAgLy8gY2hlY2sgdmFsdWUgdm90ZWQgZm9yCiAgfSBlbHNlIGZhbHNlCiAgCiAgdmFsIGJhbGxvdEJveGVzID0gSU5QVVRTLmZpbHRlcihpc1ZhbGlkQmFsbG90KQogIAogIHZhbCB2b3Rlc0NvdW50ID0gYmFsbG90Qm94ZXMuZm9sZCgwTCwgeyhhY2N1bTogTG9uZywgYjogQm94KSA9PiBhY2N1bSArIGIudG9rZW5zKDApLl8yfSkKICAKICBzaWdtYVByb3AodmFsaWRQb29sSW4gJiYgdmFsaWRQb29sT3V0ICYmIHZhbGlkVXBkYXRlT3V0ICYmIHZvdGVzQ291bnQgPj0gbWluVm90ZXMpICAKfQ==
    const P2S: &'static str = "Huhjcxyn5T7Tmn6QEJZqAsbu8FmYTUdMhvJo3v2gZCDM86q18udw9HhbKRXJDuindgnvgD6YvMf82Q89ephtD7ynHBfdwwq9mBAYWzoYTUG7j6QZEfhj3sj7QdQJK6woyic5m1G6d36ySVMJpKNXLihdW9ouwhiHL293otb7FE5tuYvq5Djrtj9oiGnVSFLvdnj2PXx45EkPUTpMABxcJ9wMxJZAi26Jd6yHifcEPhX9mBRtZuJjaNn9QFEK9EVxpxWpP3BFK8Ezk85AZo94zTBzMLD9CCJCdrcuxy1V1KwRgCukK6j6hKEkpVttcMQPCLoJM19NRsbnsmHQxZyx5Jr9B9vDNBAzPAEPBCTAQfw7uo3MyjrvQ3rjzFgGtpBmFdUeWKkQNmRgZcA8GkwA8bjL95WXFKJ9DdCZQnJYV5GYXzWXYgbQjLHCJ1jWZWRZN5Y7dW1VmxR51rToZRoKx";
    const POOL_NFT_INDEX: usize = 4;
    const BALLOT_TOKEN_INDEX: usize = 7;
    const MIN_VOTES_INDEX: usize = 11;

    pub fn new() -> Self {
        let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
        let addr = encoder.parse_address_from_str(Self::P2S).unwrap();
        let ergo_tree = addr.script().unwrap();
        Self::from_ergo_tree(ergo_tree).unwrap()
    }
    pub fn from_ergo_tree(ergo_tree: ErgoTree) -> Result<Self, UpdateContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        if ergo_tree
            .get_constant(Self::POOL_NFT_INDEX)
            .map_err(|_| UpdateContractError::NoPoolNftId)?
            .ok_or(UpdateContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(UpdateContractError::NoPoolNftId);
        };

        if ergo_tree
            .get_constant(Self::BALLOT_TOKEN_INDEX)
            .map_err(|_| UpdateContractError::NoBallotTokenId)?
            .ok_or(UpdateContractError::NoBallotTokenId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(UpdateContractError::NoBallotTokenId);
        };
        if ergo_tree
            .get_constant(Self::MIN_VOTES_INDEX)
            .map_err(|_| UpdateContractError::MinVotesError)?
            .ok_or(UpdateContractError::MinVotesError)?
            .tpe
            != SType::SInt
        {
            return Err(UpdateContractError::MinVotesError);
        };
        Ok(Self { ergo_tree })
    }

    pub fn min_votes(&self) -> i32 {
        let vote_constant = self
            .ergo_tree
            .get_constant(Self::MIN_VOTES_INDEX)
            .unwrap()
            .unwrap();
        if let Literal::Int(votes) = vote_constant.v {
            votes
        } else {
            panic!(
                "update: minimum votes is wrong type, expected SInt, found {:?}",
                vote_constant.tpe
            );
        }
    }
    pub fn with_min_votes(self, min_votes: i32) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::MIN_VOTES_INDEX, min_votes.into())
            .unwrap();
        UpdateContract { ergo_tree: tree }
    }
    pub fn pool_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(Self::POOL_NFT_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }
    pub fn with_pool_nft_token_id(self, token_id: TokenId) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::POOL_NFT_INDEX, token_id.clone().into())
            .unwrap();
        Self { ergo_tree: tree }
    }

    pub fn ballot_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(Self::BALLOT_TOKEN_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }
    pub fn with_ballot_token_id(self, token_id: TokenId) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::BALLOT_TOKEN_INDEX, token_id.clone().into())
            .unwrap();
        Self { ergo_tree: tree }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_parsing() {
        let c = UpdateContract::new();
        assert_eq!(
            c.pool_nft_token_id(),
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc").unwrap()
        );
        assert_eq!(
            c.ballot_token_id(),
            TokenId::from_base64("P0QoRy1LYVBkU2dWa1lwM3M2djl5JEImRSlIQE1iUWU").unwrap()
        );
        assert_eq!(c.min_votes(), 6);
    }
    #[test]
    fn test_constant_update() {
        let new_pool_nft_token_id =
            TokenId::from_base64("RYTLYLBLU2HWBVLXM3Q2dzl6JEMmRilKQE1jUWZUalc").unwrap();
        let new_ballot_token_id =
            TokenId::from_base64("P0QORY1LYVBKU2dWa1lwM3M2djl5JEImRSlIQE1iUWU").unwrap();
        let c = UpdateContract::new()
            .with_min_votes(7)
            .with_ballot_token_id(new_ballot_token_id.clone())
            .with_pool_nft_token_id(new_pool_nft_token_id.clone());
        assert_eq!(c.min_votes(), 7);
        assert_eq!(c.ballot_token_id(), new_ballot_token_id);
        assert_eq!(c.pool_nft_token_id(), new_pool_nft_token_id);
    }
}
