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
    // from https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAodXBkYXRlIGJveCk6CiAgICAgICAgIC8vIFJlZ2lzdGVycyBlbXB0eSAKICAgICAgICAgLy8gCiAgICAgICAgIC8vIGJhbGxvdCBib3hlcyAoSW5wdXRzKQogICAgICAgICAvLyBSNCB0aGUgcHViIGtleSBvZiB2b3RlciBbR3JvdXBFbGVtZW50XSAobm90IHVzZWQgaGVyZSkKICAgICAgICAgLy8gUjUgdGhlIGNyZWF0aW9uIGhlaWdodCBvZiB0aGlzIGJveCBbSW50XQogICAgICAgICAvLyBSNiB0aGUgdmFsdWUgdm90ZWQgZm9yIFtDb2xsW0J5dGVdXSAoaGFzaCBvZiB0aGUgbmV3IHBvb2wgYm94IHNjcmlwdCkKICAgICAgICAgLy8gUjcgdGhlIHJld2FyZCB0b2tlbiBpZCBpbiBuZXcgYm94IAogICAgICAgICAvLyBSOCB0aGUgbnVtYmVyIG9mIHJld2FyZCB0b2tlbnMgaW4gbmV3IGJveCAKICAgICAgIAogICAgICAgICB2YWwgcG9vbE5GVCA9IGZyb21CYXNlNjQoIlJ5dExZbEJsVTJoV2JWbHhNM1EyZHpsNkpFTW1SaWxLUUUxalVXWlVhbGM9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsIAogICAgICAKICAgICAgICAgdmFsIGJhbGxvdFRva2VuSWQgPSBmcm9tQmFzZTY0KCJQMFFvUnkxTFlWQmtVMmRXYTFsd00zTTJkamw1SkVJbVJTbElRRTFpVVdVPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKICAgICAgIAogICAgICAgICB2YWwgbWluVm90ZXMgPSA2IC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbAogICAgICAgICAKICAgICAgICAgdmFsIHBvb2xJbiA9IElOUFVUUygwKSAvLyBwb29sIGJveCBpcyAxc3QgaW5wdXQKICAgICAgICAgdmFsIHBvb2xPdXQgPSBPVVRQVVRTKDApIC8vIGNvcHkgb2YgcG9vbCBib3ggaXMgdGhlIDFzdCBvdXRwdXQKICAgICAgIAogICAgICAgICB2YWwgdXBkYXRlQm94T3V0ID0gT1VUUFVUUygxKSAvLyBjb3B5IG9mIHRoaXMgYm94IGlzIHRoZSAybmQgb3V0cHV0CiAgICAgICAKICAgICAgICAgLy8gY29tcHV0ZSB0aGUgaGFzaCBvZiB0aGUgcG9vbCBvdXRwdXQgYm94LiBUaGlzIHNob3VsZCBiZSB0aGUgdmFsdWUgdm90ZWQgZm9yCiAgICAgICAgIHZhbCBwb29sT3V0SGFzaCA9IGJsYWtlMmIyNTYocG9vbE91dC5wcm9wb3NpdGlvbkJ5dGVzKQogICAgICAgICB2YWwgcmV3YXJkVG9rZW5JZCA9IHBvb2xPdXQudG9rZW5zKDEpLl8xCiAgICAgICAgIHZhbCByZXdhcmRBbXQgPSBwb29sT3V0LnRva2VucygxKS5fMgogICAgICAgICAKICAgICAgICAgdmFsIHZhbGlkUG9vbEluID0gcG9vbEluLnRva2VucygwKS5fMSA9PSBwb29sTkZUCiAgICAgICAgIAogICAgICAgICB2YWwgdmFsaWRQb29sT3V0ID0gcG9vbEluLnByb3Bvc2l0aW9uQnl0ZXMgIT0gcG9vbE91dC5wcm9wb3NpdGlvbkJ5dGVzICAmJiAvLyBzY3JpcHQgc2hvdWxkIG5vdCBiZSBwcmVzZXJ2ZWQKICAgICAgICAgICAgICAgICAgICAgICAgICAgIHBvb2xJbi50b2tlbnMoMCkgPT0gcG9vbE91dC50b2tlbnMoMCkgICAgICAgICAgICAgICAgJiYgLy8gTkZUIHByZXNlcnZlZAogICAgICAgICAgICAgICAgICAgICAgICAgICAgcG9vbEluLmNyZWF0aW9uSW5mby5fMSA9PSBwb29sT3V0LmNyZWF0aW9uSW5mby5fMSAgICAmJiAvLyBjcmVhdGlvbiBoZWlnaHQgcHJlc2VydmVkCiAgICAgICAgICAgICAgICAgICAgICAgICAgICBwb29sSW4udmFsdWUgPT0gcG9vbE91dC52YWx1ZSAgICAgICAgICAgICAgICAgICAgICAgICYmIC8vIHZhbHVlIHByZXNlcnZlZCAKICAgICAgICAgICAgICAgICAgICAgICAgICAgIHBvb2xJbi5SNFtMb25nXSA9PSBwb29sT3V0LlI0W0xvbmddICAgICAgICAgICAgICAgICAgJiYgLy8gcmF0ZSBwcmVzZXJ2ZWQgIAogICAgICAgICAgICAgICAgICAgICAgICAgICAgcG9vbEluLlI1W0ludF0gPT0gcG9vbE91dC5SNVtJbnRdICAgICAgICAgICAgICAgICAgICAmJiAvLyBjb3VudGVyIHByZXNlcnZlZAogICAgICAgICAgICAgICAgICAgICAgICAgICAgISAocG9vbE91dC5SNltBbnldLmlzRGVmaW5lZCkKICAgICAgIAogICAgICAgICAKICAgICAgICAgdmFsIHZhbGlkVXBkYXRlT3V0ID0gdXBkYXRlQm94T3V0LnRva2VucyA9PSBTRUxGLnRva2VucyAgICAgICAgICAgICAgICAgICAgICYmCiAgICAgICAgICAgICAgICAgICAgICAgICAgICAgIHVwZGF0ZUJveE91dC5wcm9wb3NpdGlvbkJ5dGVzID09IFNFTEYucHJvcG9zaXRpb25CeXRlcyAmJgogICAgICAgICAgICAgICAgICAgICAgICAgICAgICB1cGRhdGVCb3hPdXQudmFsdWUgPj0gU0VMRi52YWx1ZSAgICAgICAgICAgICAgICAgICAgICAgJiYKICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgdXBkYXRlQm94T3V0LmNyZWF0aW9uSW5mby5fMSA+IFNFTEYuY3JlYXRpb25JbmZvLl8xICAgICYmCiAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICEgKHVwZGF0ZUJveE91dC5SNFtBbnldLmlzRGVmaW5lZCkgCiAgICAgICAKICAgICAgICAgZGVmIGlzVmFsaWRCYWxsb3QoYjpCb3gpID0gaWYgKGIudG9rZW5zLnNpemUgPiAwKSB7CiAgICAgICAgICAgYi50b2tlbnMoMCkuXzEgPT0gYmFsbG90VG9rZW5JZCAgICAgICAmJgogICAgICAgICAgIGIuUjVbSW50XS5nZXQgPT0gU0VMRi5jcmVhdGlvbkluZm8uXzEgJiYgLy8gZW5zdXJlIHZvdGUgY29ycmVzcG9uZHMgdG8gdGhpcyBib3ggYnkgY2hlY2tpbmcgY3JlYXRpb24gaGVpZ2h0CiAgICAgICAgICAgYi5SNltDb2xsW0J5dGVdXS5nZXQgPT0gcG9vbE91dEhhc2ggICAmJiAvLyBjaGVjayBwcm9wb3NpdGlvbiB2b3RlZCBmb3IKICAgICAgICAgICBiLlI3W0NvbGxbQnl0ZV1dLmdldCA9PSByZXdhcmRUb2tlbklkICYmIC8vIGNoZWNrIHJld2FyZFRva2VuSWQgdm90ZWQgZm9yCiAgICAgICAgICAgYi5SOFtMb25nXS5nZXQgPT0gcmV3YXJkQW10ICAgICAgICAgICAgICAvLyBjaGVjayByZXdhcmRUb2tlbkFtdCB2b3RlZCBmb3IKICAgICAgICAgfSBlbHNlIGZhbHNlCiAgICAgICAgIAogICAgICAgICB2YWwgYmFsbG90Qm94ZXMgPSBJTlBVVFMuZmlsdGVyKGlzVmFsaWRCYWxsb3QpCiAgICAgICAgIAogICAgICAgICB2YWwgdm90ZXNDb3VudCA9IGJhbGxvdEJveGVzLmZvbGQoMEwsIHsoYWNjdW06IExvbmcsIGI6IEJveCkgPT4gYWNjdW0gKyBiLnRva2VucygwKS5fMn0pCiAgICAgICAgIAogICAgICAgICBzaWdtYVByb3AodmFsaWRQb29sSW4gJiYgdmFsaWRQb29sT3V0ICYmIHZhbGlkVXBkYXRlT3V0ICYmIHZvdGVzQ291bnQgPj0gbWluVm90ZXMpICAKICAgICAgIH0KICAgICAgIA==
    const P2S: &'static str = "RGQjcwtwcPBVwTFZMaGyo471kgwcwtMjrUy41RqWhAtY2ovdKAQ2Ce3cUaF6S7LGMrV3boM5rGKR5K2vjyheDXtVuEoUpZefQ2qa7H8MPBaYfAWqttNpyp5A1GfYviWfSbbEsbUSptgUMHH9MTLCnkvQdfxtC9HvKX8gJdaJBhEF4KHUBDVcsuMX33vcqi7Y5voEjunnmgvbpcYBG6HAkZtz15uXh1TskFpumFDgqwMbExapeRRXbq3EjuVAqEeoibastYMLrZ1evAq1bZP9mFoQRd15kUgBHvRQLwHJzdcRSz1pCM6UXTsna599VQBCiqKRZ9iCDffeUGuvjJBgzm5gouMCpaEc6LJn5Z2ta5MFAvQpd1MhtvTBL6X6NFKbYJxNWFK7igqbf9nDtbkcrUjRD2LKeqEapNRbLnxyMd6Dd5nMKZLuthkgsK3BSmN4YKh2S94wNE5PRDM1FULTg1RC7tFvRV5aKmcKD25M7qYwXwLqWoRPCk7C8CqCdSHT2cJTM3RAx6xSbt5Cq";
    const POOL_NFT_INDEX: usize = 5;
    const BALLOT_TOKEN_INDEX: usize = 9;
    const MIN_VOTES_INDEX: usize = 13;

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
