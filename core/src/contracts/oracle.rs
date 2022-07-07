use derive_more::From;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use thiserror::Error;

use crate::oracle_config::OracleContractParameters;

#[derive(Clone)]
pub struct OracleContract {
    ergo_tree: ErgoTree,
    pool_nft_index: usize,
}

#[derive(Debug, From, Error)]
pub enum OracleContractError {
    #[error("oracle contract: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("oracle contract: unknown pool NFT in box")]
    UnknownPoolNftId,
    #[error("oracle contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("oracle contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("oracle contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl OracleContract {
    // via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAob3JhY2xlIGJveCkKICAvLyAgIFI0IHB1YmxpYyBrZXkgKEdyb3VwRWxlbWVudCkgCiAgLy8gICBSNSBlcG9jaCBjb3VudGVyIG9mIGN1cnJlbnQgZXBvY2ggKEludCkKICAvLyAgIFI2IGRhdGEgcG9pbnQgKExvbmcpIG9yIGVtcHR5CgogIC8vICAgdG9rZW5zKDApIG9yYWNsZSB0b2tlbiAob25lKQogIC8vICAgdG9rZW5zKDEpIHJld2FyZCB0b2tlbnMgY29sbGVjdGVkIChvbmUgb3IgbW9yZSkgCiAgLy8gICAKICAvLyAgIFdoZW4gcHVibGlzaGluZyBhIGRhdGFwb2ludCwgdGhlcmUgbXVzdCBiZSBhdCBsZWFzdCBvbmUgcmV3YXJkIHRva2VuIGF0IGluZGV4IDEgCiAgLy8gIAogIC8vICAgV2Ugd2lsbCBjb25uZWN0IHRoaXMgYm94IHRvIHBvb2wgTkZUIGluIGlucHV0ICMwIChhbmQgbm90IHRoZSByZWZyZXNoIE5GVCBpbiBpbnB1dCAjMSkKICAvLyAgIFRoaXMgd2F5LCB3ZSBjYW4gY29udGludWUgdG8gdXNlIHRoZSBzYW1lIGJveCBhZnRlciB1cGRhdGluZyBwb29sCiAgLy8gICBUaGlzICpjb3VsZCogYWxsb3cgdGhlIG9yYWNsZSBib3ggdG8gYmUgc3BlbnQgZHVyaW5nIGFuIHVwZGF0ZQogIC8vICAgSG93ZXZlciwgdGhpcyBpcyBub3QgYW4gaXNzdWUgYmVjYXVzZSB0aGUgdXBkYXRlIGNvbnRyYWN0IGVuc3VyZXMgdGhhdCB0b2tlbnMgYW5kIHJlZ2lzdGVycyAoZXhjZXB0IHNjcmlwdCkgb2YgdGhlIHBvb2wgYm94IGFyZSBwcmVzZXJ2ZWQKCiAgLy8gICBQcml2YXRlIGtleSBob2xkZXIgY2FuIGRvIGZvbGxvd2luZyB0aGluZ3M6CiAgLy8gICAgIDEuIENoYW5nZSBncm91cCBlbGVtZW50IChwdWJsaWMga2V5KSBzdG9yZWQgaW4gUjQKICAvLyAgICAgMi4gU3RvcmUgYW55IHZhbHVlIG9mIHR5cGUgaW4gb3IgZGVsZXRlIGFueSB2YWx1ZSBmcm9tIFI0IHRvIFI5IAogIC8vICAgICAzLiBTdG9yZSBhbnkgdG9rZW4gb3Igbm9uZSBhdCAybmQgaW5kZXggCgogIC8vICAgSW4gb3JkZXIgdG8gY29ubmVjdCB0aGlzIG9yYWNsZSBib3ggdG8gYSBkaWZmZXJlbnQgcmVmcmVzaE5GVCBhZnRlciBhbiB1cGRhdGUsIAogIC8vICAgdGhlIG9yYWNsZSBzaG91bGQga2VlcCBhdCBsZWFzdCBvbmUgbmV3IHJld2FyZCB0b2tlbiBhdCBpbmRleCAxIHdoZW4gcHVibGlzaGluZyBkYXRhLXBvaW50CiAgCiAgdmFsIHBvb2xORlQgPSBmcm9tQmFzZTY0KCJSeXRMWWxCbFUyaFdiVmx4TTNRMmR6bDZKRU1tUmlsS1FFMWpVV1pVYWxjPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKICAKICB2YWwgb3RoZXJUb2tlbklkID0gSU5QVVRTKDApLnRva2VucygwKS5fMQogIAogIHZhbCBtaW5TdG9yYWdlUmVudCA9IDEwMDAwMDAwTAogIHZhbCBzZWxmUHViS2V5ID0gU0VMRi5SNFtHcm91cEVsZW1lbnRdLmdldAogIHZhbCBvdXRJbmRleCA9IGdldFZhcltJbnRdKDApLmdldAogIHZhbCBvdXRwdXQgPSBPVVRQVVRTKG91dEluZGV4KQogIAogIHZhbCBpc1NpbXBsZUNvcHkgPSBvdXRwdXQudG9rZW5zKDApID09IFNFTEYudG9rZW5zKDApICAgICAgICAgICAgICAgICYmIC8vIG9yYWNsZSB0b2tlbiBpcyBwcmVzZXJ2ZWQKICAgICAgICAgICAgICAgICAgICAgb3V0cHV0LnByb3Bvc2l0aW9uQnl0ZXMgPT0gU0VMRi5wcm9wb3NpdGlvbkJ5dGVzICAmJiAvLyBzY3JpcHQgcHJlc2VydmVkCiAgICAgICAgICAgICAgICAgICAgIG91dHB1dC5SNFtHcm91cEVsZW1lbnRdLmlzRGVmaW5lZCAgICAgICAgICAgICAgICAgJiYgLy8gb3V0cHV0IG11c3QgaGF2ZSBhIHB1YmxpYyBrZXkgKG5vdCBuZWNlc3NhcmlseSB0aGUgc2FtZSkKICAgICAgICAgICAgICAgICAgICAgb3V0cHV0LnZhbHVlID49IG1pblN0b3JhZ2VSZW50ICAgICAgICAgICAgICAgICAgICAgICAvLyBlbnN1cmUgc3VmZmljaWVudCBFcmdzIHRvIGVuc3VyZSBubyBnYXJiYWdlIGNvbGxlY3Rpb24KICAgICAgICAgICAgICAgICAgICAgCiAgdmFsIGNvbGxlY3Rpb24gPSBvdGhlclRva2VuSWQgPT0gcG9vbE5GVCAgICAgICAgICAgICAgICAgICAgJiYgLy8gZmlyc3QgaW5wdXQgbXVzdCBiZSBwb29sIGJveAogICAgICAgICAgICAgICAgICAgb3V0cHV0LnRva2VucygxKS5fMSA9PSBTRUxGLnRva2VucygxKS5fMSAgICYmIC8vIHJld2FyZCB0b2tlbklkIGlzIHByZXNlcnZlZCAob3JhY2xlIHNob3VsZCBlbnN1cmUgdGhpcyBjb250YWlucyBhIHJld2FyZCB0b2tlbikKICAgICAgICAgICAgICAgICAgIG91dHB1dC50b2tlbnMoMSkuXzIgPiBTRUxGLnRva2VucygxKS5fMiAgICAmJiAvLyBhdCBsZWFzdCBvbmUgcmV3YXJkIHRva2VuIG11c3QgYmUgYWRkZWQgCiAgICAgICAgICAgICAgICAgICBvdXRwdXQuUjRbR3JvdXBFbGVtZW50XS5nZXQgPT0gc2VsZlB1YktleSAgJiYgLy8gZm9yIGNvbGxlY3Rpb24gcHJlc2VydmUgcHVibGljIGtleQogICAgICAgICAgICAgICAgICAgb3V0cHV0LnZhbHVlID49IFNFTEYudmFsdWUgICAgICAgICAgICAgICAgICYmIC8vIG5hbm9FcmdzIHZhbHVlIHByZXNlcnZlZAogICAgICAgICAgICAgICAgICAgISAob3V0cHV0LlI1W0FueV0uaXNEZWZpbmVkKSAgICAgICAgICAgICAgICAgIC8vIG5vIG1vcmUgcmVnaXN0ZXJzOyBwcmV2ZW50cyBib3ggZnJvbSBiZWluZyByZXVzZWQgYXMgYSB2YWxpZCBkYXRhLXBvaW50CgogIHZhbCBvd25lciA9IHByb3ZlRGxvZyhzZWxmUHViS2V5KSAgCgogIC8vIG93bmVyIGNhbiBjaG9vc2UgdG8gdHJhbnNmZXIgdG8gYW5vdGhlciBwdWJsaWMga2V5IGJ5IHNldHRpbmcgZGlmZmVyZW50IHZhbHVlIGluIFI0CiAgaXNTaW1wbGVDb3B5ICYmIChvd25lciB8fCBjb2xsZWN0aW9uKSAKfQ==
    //
    //const P2S: &'static str = "2vTHJzWVd7ryXrP3fH9KfEFGzS8XFdVY99xXuxMPt664HurrUn3e8y3W1wTQDVZsDi9TDeZdun2XEr3pcipGmKdmciSADmKn32Cs8YuPLNp4zaBZNo6m6NG8tz3zznb56nRCrz5VDDjxYTsQ92DqhtQmG3m7H6zbtNHLzJjf7x9ZSD3vNWRL6e7usRjfm1diob8bdizsbJM7wNDzLZYhshHScEkWse9MQKgMDN4pYb1vQLR1PmvUnpsRAjRYwNBs3ZjJoqdSpN6jbjfSJsrgEhBANbnCZxP3dKBr";

    //pub const POOL_NFT_INDEX: usize = 5;

    pub fn new(parameters: &OracleContractParameters) -> Result<Self, OracleContractError> {
        let ergo_tree = parameters.p2s.address().script()?.with_constant(
            parameters.pool_nft_index,
            parameters.pool_nft_token_id.clone().into(),
        )?;
        let contract = Self::from_ergo_tree(ergo_tree, parameters)?;
        Ok(contract)
    }

    /// Create new contract from existing ergo tree, returning error if parameters differ.
    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        parameters: &OracleContractParameters,
    ) -> Result<Self, OracleContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        let m = ergo_tree
            .get_constant(parameters.pool_nft_index)
            .map_err(|_| OracleContractError::NoPoolNftId)?
            .ok_or(OracleContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>();
        match m {
            Ok(token_id) => {
                if token_id != parameters.pool_nft_token_id {
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

#[cfg(test)]
mod tests {
    use crate::pool_commands::test_utils::make_oracle_contract_parameters;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let parameters = make_oracle_contract_parameters();
        let pool_nft_token_id = parameters.pool_nft_token_id.clone();
        let c = OracleContract::new(&parameters).unwrap();
        assert_eq!(c.pool_nft_token_id(), pool_nft_token_id,);
    }
}
