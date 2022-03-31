use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::Constant;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

pub struct PoolContract {
    ergo_tree: ErgoTree,
    refresh_nft_token_id: TokenId,
}

impl PoolContract {
    // via
    // https://wallet.plutomonkey.com/p2s/?source=ewogIC8vIFRoaXMgYm94IChwb29sIGJveCkKICAvLyAgIGVwb2NoIHN0YXJ0IGhlaWdodCBpcyBzdG9yZWQgaW4gY3JlYXRpb24gSGVpZ2h0IChSMykKICAvLyAgIFI0IEN1cnJlbnQgZGF0YSBwb2ludCAoTG9uZykKICAvLyAgIFI1IEN1cnJlbnQgZXBvY2ggY291bnRlciAoSW50KQogIC8vICAgdG9rZW5zKDApIHBvb2wgdG9rZW4gKE5GVCkKICAKICB2YWwgb3RoZXJUb2tlbklkID0gSU5QVVRTKDEpLnRva2VucygwKS5fMQogIHZhbCByZWZyZXNoTkZUID0gZnJvbUJhc2U2NCgiVkdwWGJscHlOSFUzZUNGQkpVUXFSeTFMWVU1a1VtZFZhMWh3TW5NMWRqZz0iKSAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwKICB2YWwgdXBkYXRlTkZUID0gZnJvbUJhc2U2NCgiWWxGbFZHaFhiVnB4TkhRM2R5RjZKVU1xUmkxS1FFNWpVbVpWYWxodU1uST0iKSAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwKCiAgc2lnbWFQcm9wKG90aGVyVG9rZW5JZCA9PSByZWZyZXNoTkZUIHx8IG90aGVyVG9rZW5JZCA9PSB1cGRhdGVORlQpCn0=
    const P2S: &'static str = "PViBL5acX6PoP6BQPsYtyNzW9aPXwxpRaUkXo4nE7RkxcBbZXJECUEBQm4g3MQCb2QsQALqPkrDN9TvsKuQkChF8sZSfnH5fifgKAkXhW8ifAcAE1qA67n9mabB3Mb2R8xT2v3SN49eN8mQ8HN95";

    pub fn new() -> Self {
        let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
        let addr = encoder.parse_address_from_str(Self::P2S).unwrap();
        let ergo_tree = addr.script().unwrap();
        dbg!((0..ergo_tree.constants_len().unwrap())
            .map(|i| (i, ergo_tree.get_constant(i).unwrap().unwrap()))
            .collect::<Vec<(usize, Constant)>>());
        let refresh_nft_token_id: TokenId = ergo_tree
            .get_constant(2)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap();

        assert_eq!(
            refresh_nft_token_id,
            TokenId::from_base64("VGpXblpyNHU3eCFBJUQqRy1LYU5kUmdVa1hwMnM1djg=").unwrap()
        );

        Self {
            ergo_tree,
            refresh_nft_token_id,
        }
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }
}
