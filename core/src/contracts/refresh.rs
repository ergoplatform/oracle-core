use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

pub struct RefreshContract {
    ergo_tree: ErgoTree,
    min_data_points: u32,
    max_deviation_percent: u32,
    pool_nft_token_id: TokenId,
    oracle_nft_token_id: TokenId,
}

impl RefreshContract {
    // via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAocmVmcmVzaCBib3gpCiAgLy8gICB0b2tlbnMoMCkgcmV3YXJkIHRva2VucyB0byBiZSBlbWl0dGVkIChzZXZlcmFsKSAKICAvLyAgIAogIC8vICAgV2hlbiBpbml0aWFsaXppbmcgdGhlIGJveCwgdGhlcmUgbXVzdCBiZSBvbmUgcmV3YXJkIHRva2VuLiBXaGVuIGNsYWltaW5nIHJld2FyZCwgb25lIHRva2VuIG11c3QgYmUgbGVmdCB1bmNsYWltZWQgICAKICAKICB2YWwgb3JhY2xlVG9rZW5JZCA9IGZyb21CYXNlNjQoIktrY3RTbUZPWkZKblZXdFljREp6TlhZNGVTOUNQMFVvU0N0TllsQmxVMmc9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIHBvb2xORlQgPSBmcm9tQmFzZTY0KCJSeXRMWWxCbFUyaFdiVmx4TTNRMmR6bDZKRU1tUmlsS1FFMWpVV1pVYWxjPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKICB2YWwgZXBvY2hMZW5ndGggPSAzMCAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwKICB2YWwgbWluRGF0YVBvaW50cyA9IDQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIGJ1ZmZlciA9IDQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIG1heERldmlhdGlvblBlcmNlbnQgPSA1IC8vIHBlcmNlbnQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCgogIHZhbCBtaW5TdGFydEhlaWdodCA9IEhFSUdIVCAtIGVwb2NoTGVuZ3RoCiAgdmFsIHNwZW5kZXJJbmRleCA9IGdldFZhcltJbnRdKDApLmdldCAvLyB0aGUgaW5kZXggb2YgdGhlIGRhdGEtcG9pbnQgYm94IChOT1QgaW5wdXQhKSBiZWxvbmdpbmcgdG8gc3BlbmRlciAgICAKICAgIAogIHZhbCBwb29sSW4gPSBJTlBVVFMoMCkKICB2YWwgcG9vbE91dCA9IE9VVFBVVFMoMCkKICB2YWwgc2VsZk91dCA9IE9VVFBVVFMoMSkKCiAgZGVmIGlzVmFsaWREYXRhUG9pbnQoYjogQm94KSA9IGlmIChiLlI2W0xvbmddLmlzRGVmaW5lZCkgewogICAgYi5jcmVhdGlvbkluZm8uXzEgICAgPj0gbWluU3RhcnRIZWlnaHQgJiYgIC8vIGRhdGEgcG9pbnQgbXVzdCBub3QgYmUgdG9vIG9sZAogICAgYi50b2tlbnMoMCkuXzEgICAgICAgPT0gb3JhY2xlVG9rZW5JZCAgJiYgLy8gZmlyc3QgdG9rZW4gaWQgbXVzdCBiZSBvZiBvcmFjbGUgdG9rZW4KICAgIGIuUjVbSW50XS5nZXQgICAgICAgID09IHBvb2xJbi5SNVtJbnRdLmdldCAvLyBpdCBtdXN0IGNvcnJlc3BvbmQgdG8gdGhpcyBlcG9jaAogIH0gZWxzZSBmYWxzZSAKICAgICAgICAgIAogIHZhbCBkYXRhUG9pbnRzID0gSU5QVVRTLmZpbHRlcihpc1ZhbGlkRGF0YVBvaW50KSAgICAKICB2YWwgcHViS2V5ID0gZGF0YVBvaW50cyhzcGVuZGVySW5kZXgpLlI0W0dyb3VwRWxlbWVudF0uZ2V0CgogIHZhbCBlbm91Z2hEYXRhUG9pbnRzID0gZGF0YVBvaW50cy5zaXplID49IG1pbkRhdGFQb2ludHMgICAgCiAgdmFsIHJld2FyZEVtaXR0ZWQgPSBkYXRhUG9pbnRzLnNpemUgKiAyIC8vIG9uZSBleHRyYSB0b2tlbiBmb3IgZWFjaCBjb2xsZWN0ZWQgYm94IGFzIHJld2FyZCB0byBjb2xsZWN0b3IgICAKICB2YWwgZXBvY2hPdmVyID0gcG9vbEluLmNyZWF0aW9uSW5mby5fMSA8IG1pblN0YXJ0SGVpZ2h0CiAgICAgICAKICB2YWwgc3RhcnREYXRhID0gMUwgLy8gd2UgZG9uJ3QgYWxsb3cgMCBkYXRhIHBvaW50cwogIHZhbCBzdGFydFN1bSA9IDBMIAogIC8vIHdlIGV4cGVjdCBkYXRhLXBvaW50cyB0byBiZSBzb3J0ZWQgaW4gSU5DUkVBU0lORyBvcmRlcgogIAogIHZhbCBsYXN0U29ydGVkU3VtID0gZGF0YVBvaW50cy5mb2xkKChzdGFydERhdGEsICh0cnVlLCBzdGFydFN1bSkpLCB7CiAgICAgICAgKHQ6IChMb25nLCAoQm9vbGVhbiwgTG9uZykpLCBiOiBCb3gpID0+CiAgICAgICAgICAgdmFsIGN1cnJEYXRhID0gYi5SNltMb25nXS5nZXQKICAgICAgICAgICB2YWwgcHJldkRhdGEgPSB0Ll8xCiAgICAgICAgICAgdmFsIHdhc1NvcnRlZCA9IHQuXzIuXzEgCiAgICAgICAgICAgdmFsIG9sZFN1bSA9IHQuXzIuXzIKICAgICAgICAgICB2YWwgbmV3U3VtID0gb2xkU3VtICsgY3VyckRhdGEgIC8vIHdlIGRvbid0IGhhdmUgdG8gd29ycnkgYWJvdXQgb3ZlcmZsb3csIGFzIGl0IGNhdXNlcyBzY3JpcHQgdG8gZmFpbAoKICAgICAgICAgICB2YWwgaXNTb3J0ZWQgPSB3YXNTb3J0ZWQgJiYgcHJldkRhdGEgPD0gY3VyckRhdGEgCgogICAgICAgICAgIChjdXJyRGF0YSwgKGlzU29ydGVkLCBuZXdTdW0pKQogICAgfQogICkKIAogIHZhbCBsYXN0RGF0YSA9IGxhc3RTb3J0ZWRTdW0uXzEKICB2YWwgaXNTb3J0ZWQgPSBsYXN0U29ydGVkU3VtLl8yLl8xCiAgdmFsIHN1bSA9IGxhc3RTb3J0ZWRTdW0uXzIuXzIKICB2YWwgYXZlcmFnZSA9IHN1bSAvIGRhdGFQb2ludHMuc2l6ZSAKCiAgdmFsIG1heERlbHRhID0gbGFzdERhdGEgKiBtYXhEZXZpYXRpb25QZXJjZW50IC8gMTAwICAgICAgICAgIAogIHZhbCBmaXJzdERhdGEgPSBkYXRhUG9pbnRzKDApLlI2W0xvbmddLmdldAoKICBwcm92ZURsb2cocHViS2V5KSAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICYmCiAgZXBvY2hPdmVyICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAmJiAKICBlbm91Z2hEYXRhUG9pbnRzICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICYmICAgIAogIGlzU29ydGVkICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgJiYKICBsYXN0RGF0YSAtIGZpcnN0RGF0YSAgICAgPD0gbWF4RGVsdGEgICAgICAgICAgICAgICAgICAgICAgICAgICYmIAogIHBvb2xJbi50b2tlbnMoMCkuXzEgICAgICA9PSBwb29sTkZUICAgICAgICAgICAgICAgICAgICAgICAgICAgJiYKICBwb29sT3V0LnRva2VucyAgICAgICAgICAgPT0gcG9vbEluLnRva2VucyAgICAgICAgICAgICAgICAgICAgICYmIC8vIHByZXNlcnZlIHBvb2wgdG9rZW5zCiAgcG9vbE91dC5SNFtMb25nXS5nZXQgICAgID09IGF2ZXJhZ2UgICAgICAgICAgICAgICAgICAgICAgICAgICAmJiAvLyByYXRlCiAgcG9vbE91dC5SNVtJbnRdLmdldCAgICAgID09IHBvb2xJbi5SNVtJbnRdLmdldCArIDEgICAgICAgICAgICAmJiAvLyBjb3VudGVyCiAgcG9vbE91dC5wcm9wb3NpdGlvbkJ5dGVzID09IHBvb2xJbi5wcm9wb3NpdGlvbkJ5dGVzICAgICAgICAgICAmJiAvLyBwcmVzZXJ2ZSBwb29sIHNjcmlwdAogIHBvb2xPdXQudmFsdWUgICAgICAgICAgICA+PSBwb29sSW4udmFsdWUgICAgICAgICAgICAgICAgICAgICAgJiYKICBwb29sT3V0LmNyZWF0aW9uSW5mby5fMSAgPj0gSEVJR0hUIC0gYnVmZmVyICAgICAgICAgICAgICAgICAgICYmIC8vIGVuc3VyZSB0aGF0IG5ldyBib3ggaGFzIGNvcnJlY3Qgc3RhcnQgZXBvY2ggaGVpZ2h0CiAgc2VsZk91dC50b2tlbnMoMCkgICAgICAgID09IFNFTEYudG9rZW5zKDApICAgICAgICAgICAgICAgICAgICAmJiAvLyByZWZyZXNoIE5GVCBwcmVzZXJ2ZWQKICBzZWxmT3V0LnRva2VucygxKS5fMSAgICAgPT0gU0VMRi50b2tlbnMoMSkuXzEgICAgICAgICAgICAgICAgICYmIC8vIHJld2FyZCB0b2tlbiBpZCBwcmVzZXJ2ZWQKICBzZWxmT3V0LnRva2VucygxKS5fMiAgICAgPj0gU0VMRi50b2tlbnMoMSkuXzIgLSByZXdhcmRFbWl0dGVkICYmIC8vIHJld2FyZCB0b2tlbiBhbW91bnQgY29ycmVjdGx5IHJlZHVjZWQKICBzZWxmT3V0LnByb3Bvc2l0aW9uQnl0ZXMgPT0gU0VMRi5wcm9wb3NpdGlvbkJ5dGVzICAgICAgICAgICAgICYmIC8vIHNjcmlwdCBwcmVzZXJ2ZWQKICBzZWxmT3V0LnZhbHVlICAgICAgICAgICAgPj0gU0VMRi52YWx1ZSAgICAgICAgICAgICAgICAgICAgICAgCn0=
    //
    const P2S: &'static str = "8A4xmqigjVZ8W4bEnZq84GtTqbTmaF9QFTR51uC8rPc66MFbHVZvh8i8C4L2Cdfezg3UtcCpMdzVCtDGpQ41nJNgvWfxrvpyRiaA8fLYqgFhZrda976SR9Znx9UYJfdRyeBrU8bqyZ5QYKuUTXs1TE2YLAyUG6jyYqPA48Nb8J6XfPytGfdX2rxHYA9rppaD3SXaxGSjFZwqM3Cn6k72jqesWA12vSrwW7PavWKjPkVxJRvtW3eTJjnDGw4GZ2BgGjCV1NXYsy4itq3W8M2DWCox1wgoz5viYVKqgALCK8Bgxj2R5u83w8x9EHKee4ZbakhwT3oF5cZ97RokgR8N2fKPbNVDDJUMctuuKt1juHAAkquiu6pqmXtuAXbWZCKz1mUUfYHNBGkhU84h1FCZWv6JAofJinVd6F3S8V8uMY1ZTeSSfwhykAiSBriGkACGo3vNr1RXDLSLCR73NwPdNJeCC4n11xsjdV19nvjg9ToLTaxUW3DaW8KdLyge1Pe6zy5phq8ohriSz5G5ib6XPNfZvmooBVFpxG66dm8V9KQNqVHeAqJ7kDForYqQYq7y1iEJRTJ5kmi9s93KksDJxk6E3Xe42BgYUBUAvp7mxfqojxJQ9B8a2xgu8niZ3W7fTprPMvzSpRtjE7sXBWwBEqAAdLU6PQuRwxreDnU2QDvX3LkaHP8PuFyqeRYQouAMPoEn3dF5RW5MYHYqgaPshubRrHJCSYBv5LW6ywdNzf7t8t5uZNhR";

    pub fn new() -> Self {
        let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
        let addr = encoder.parse_address_from_str(Self::P2S).unwrap();
        let ergo_tree = addr.script().unwrap();
        dbg!(ergo_tree.get_constants().unwrap());
        let pool_nft_token_id: TokenId = ergo_tree
            .get_constant(17)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap();
        assert_eq!(
            pool_nft_token_id,
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap()
        );
        let oracle_nft_token_id: TokenId = ergo_tree
            .get_constant(3)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap();
        assert_eq!(
            oracle_nft_token_id,
            TokenId::from_base64("KkctSmFOZFJnVWtYcDJzNXY4eS9CP0UoSCtNYlBlU2g=").unwrap()
        );

        // TODO: there is two (with the same value 4) constants
        let min_data_points = ergo_tree
            .get_constant(19)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32;
        assert_eq!(min_data_points, 4);

        let max_deviation_percent = ergo_tree
            .get_constant(14)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32;
        assert_eq!(max_deviation_percent, 5);

        Self {
            ergo_tree,
            min_data_points,
            max_deviation_percent,
            pool_nft_token_id,
            oracle_nft_token_id,
        }
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn min_data_points(&self) -> u32 {
        self.min_data_points
    }

    pub fn max_deviation_percent(&self) -> u32 {
        self.max_deviation_percent
    }

    pub fn oracle_nft_token_id(&self) -> TokenId {
        self.oracle_nft_token_id.clone()
    }
}
