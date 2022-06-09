use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;

use thiserror::Error;

#[derive(Clone)]
pub struct RefreshContract {
    ergo_tree: ErgoTree,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum RefreshContractError {
    #[error("refresh contract: failed to get pool NFT from constants")]
    NoPoolNftId,
    #[error("refresh contract: failed to get oracle token id from constants")]
    NoOracleTokenId,
    #[error("refresh contract: failed to get min data points from constants")]
    NoMinDataPoints,
    #[error("refresh contract: failed to get buffer from constants")]
    NoBuffer,
    #[error("refresh contract: failed to get max deviation percent from constants")]
    NoMaxDeviationPercent,
    #[error("refresh contract: failed to get epoch length from constants")]
    NoEpochLength,
}

impl RefreshContract {
    // v2.0a from https://github.com/scalahub/OraclePool/blob/v2/src/main/scala/oraclepool/v2a/Contracts.scala
    // compiled via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAocmVmcmVzaCBib3gpCiAgLy8gICB0b2tlbnMoMCkgcmV3YXJkIHRva2VucyB0byBiZSBlbWl0dGVkIChzZXZlcmFsKSAKICAvLyAgIAogIC8vICAgV2hlbiBpbml0aWFsaXppbmcgdGhlIGJveCwgdGhlcmUgbXVzdCBiZSBvbmUgcmV3YXJkIHRva2VuLiBXaGVuIGNsYWltaW5nIHJld2FyZCwgb25lIHRva2VuIG11c3QgYmUgbGVmdCB1bmNsYWltZWQgICAKICAKICB2YWwgb3JhY2xlVG9rZW5JZCA9IGZyb21CYXNlNjQoIktrY3RTbUZPWkZKblZXdFljREp6TlhZNGVTOUNQMFVvU0N0TllsQmxVMmc9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIHBvb2xORlQgPSBmcm9tQmFzZTY0KCJSeXRMWWxCbFUyaFdiVmx4TTNRMmR6bDZKRU1tUmlsS1FFMWpVV1pVYWxjPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKICB2YWwgZXBvY2hMZW5ndGggPSAzMCAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwKICB2YWwgbWluRGF0YVBvaW50cyA9IDQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIGJ1ZmZlciA9IDQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIG1heERldmlhdGlvblBlcmNlbnQgPSA1IC8vIHBlcmNlbnQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCgogIHZhbCBtaW5TdGFydEhlaWdodCA9IEhFSUdIVCAtIGVwb2NoTGVuZ3RoCiAgdmFsIHNwZW5kZXJJbmRleCA9IGdldFZhcltJbnRdKDApLmdldCAvLyB0aGUgaW5kZXggb2YgdGhlIGRhdGEtcG9pbnQgYm94IChOT1QgaW5wdXQhKSBiZWxvbmdpbmcgdG8gc3BlbmRlciAgICAKICAgIAogIHZhbCBwb29sSW4gPSBJTlBVVFMoMCkKICB2YWwgcG9vbE91dCA9IE9VVFBVVFMoMCkKICB2YWwgc2VsZk91dCA9IE9VVFBVVFMoMSkKCiAgZGVmIGlzVmFsaWREYXRhUG9pbnQoYjogQm94KSA9IGlmIChiLlI2W0xvbmddLmlzRGVmaW5lZCkgewogICAgYi5jcmVhdGlvbkluZm8uXzEgICAgPj0gbWluU3RhcnRIZWlnaHQgJiYgIC8vIGRhdGEgcG9pbnQgbXVzdCBub3QgYmUgdG9vIG9sZAogICAgYi50b2tlbnMoMCkuXzEgICAgICAgPT0gb3JhY2xlVG9rZW5JZCAgJiYgLy8gZmlyc3QgdG9rZW4gaWQgbXVzdCBiZSBvZiBvcmFjbGUgdG9rZW4KICAgIGIuUjVbSW50XS5nZXQgICAgICAgID09IHBvb2xJbi5SNVtJbnRdLmdldCAvLyBpdCBtdXN0IGNvcnJlc3BvbmQgdG8gdGhpcyBlcG9jaAogIH0gZWxzZSBmYWxzZSAKICAgICAgICAgIAogIHZhbCBkYXRhUG9pbnRzID0gSU5QVVRTLmZpbHRlcihpc1ZhbGlkRGF0YVBvaW50KSAgICAKICB2YWwgcHViS2V5ID0gZGF0YVBvaW50cyhzcGVuZGVySW5kZXgpLlI0W0dyb3VwRWxlbWVudF0uZ2V0CgogIHZhbCBlbm91Z2hEYXRhUG9pbnRzID0gZGF0YVBvaW50cy5zaXplID49IG1pbkRhdGFQb2ludHMgICAgCiAgdmFsIHJld2FyZEVtaXR0ZWQgPSBkYXRhUG9pbnRzLnNpemUgKiAyIC8vIG9uZSBleHRyYSB0b2tlbiBmb3IgZWFjaCBjb2xsZWN0ZWQgYm94IGFzIHJld2FyZCB0byBjb2xsZWN0b3IgICAKICB2YWwgZXBvY2hPdmVyID0gcG9vbEluLmNyZWF0aW9uSW5mby5fMSA8IG1pblN0YXJ0SGVpZ2h0CiAgICAgICAKICB2YWwgc3RhcnREYXRhID0gMUwgLy8gd2UgZG9uJ3QgYWxsb3cgMCBkYXRhIHBvaW50cwogIHZhbCBzdGFydFN1bSA9IDBMIAogIC8vIHdlIGV4cGVjdCBkYXRhLXBvaW50cyB0byBiZSBzb3J0ZWQgaW4gSU5DUkVBU0lORyBvcmRlcgogIAogIHZhbCBsYXN0U29ydGVkU3VtID0gZGF0YVBvaW50cy5mb2xkKChzdGFydERhdGEsICh0cnVlLCBzdGFydFN1bSkpLCB7CiAgICAgICAgKHQ6IChMb25nLCAoQm9vbGVhbiwgTG9uZykpLCBiOiBCb3gpID0+CiAgICAgICAgICAgdmFsIGN1cnJEYXRhID0gYi5SNltMb25nXS5nZXQKICAgICAgICAgICB2YWwgcHJldkRhdGEgPSB0Ll8xCiAgICAgICAgICAgdmFsIHdhc1NvcnRlZCA9IHQuXzIuXzEgCiAgICAgICAgICAgdmFsIG9sZFN1bSA9IHQuXzIuXzIKICAgICAgICAgICB2YWwgbmV3U3VtID0gb2xkU3VtICsgY3VyckRhdGEgIC8vIHdlIGRvbid0IGhhdmUgdG8gd29ycnkgYWJvdXQgb3ZlcmZsb3csIGFzIGl0IGNhdXNlcyBzY3JpcHQgdG8gZmFpbAoKICAgICAgICAgICB2YWwgaXNTb3J0ZWQgPSB3YXNTb3J0ZWQgJiYgcHJldkRhdGEgPD0gY3VyckRhdGEgCgogICAgICAgICAgIChjdXJyRGF0YSwgKGlzU29ydGVkLCBuZXdTdW0pKQogICAgfQogICkKIAogIHZhbCBsYXN0RGF0YSA9IGxhc3RTb3J0ZWRTdW0uXzEKICB2YWwgaXNTb3J0ZWQgPSBsYXN0U29ydGVkU3VtLl8yLl8xCiAgdmFsIHN1bSA9IGxhc3RTb3J0ZWRTdW0uXzIuXzIKICB2YWwgYXZlcmFnZSA9IHN1bSAvIGRhdGFQb2ludHMuc2l6ZSAKCiAgdmFsIG1heERlbHRhID0gbGFzdERhdGEgKiBtYXhEZXZpYXRpb25QZXJjZW50IC8gMTAwICAgICAgICAgIAogIHZhbCBmaXJzdERhdGEgPSBkYXRhUG9pbnRzKDApLlI2W0xvbmddLmdldAoKICBwcm92ZURsb2cocHViS2V5KSAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICYmCiAgZXBvY2hPdmVyICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAmJiAKICBlbm91Z2hEYXRhUG9pbnRzICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICYmICAgIAogIGlzU29ydGVkICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgJiYKICBsYXN0RGF0YSAtIGZpcnN0RGF0YSAgICAgPD0gbWF4RGVsdGEgICAgICAgICAgICAgICAgICAgICAgICAgICYmIAogIHBvb2xJbi50b2tlbnMoMCkuXzEgICAgICA9PSBwb29sTkZUICAgICAgICAgICAgICAgICAgICAgICAgICAgJiYKICBwb29sT3V0LnRva2VucygwKSAgICAgICAgPT0gcG9vbEluLnRva2VucygwKSAgICAgICAgICAgICAgICAgICYmIC8vIHByZXNlcnZlIHBvb2wgTkZUCiAgcG9vbE91dC50b2tlbnMoMSkuXzEgICAgID09IHBvb2xJbi50b2tlbnMoMSkuXzEgICAgICAgICAgICAgICAmJiAvLyByZXdhcmQgdG9rZW4gaWQgcHJlc2VydmVkCiAgcG9vbE91dC50b2tlbnMoMSkuXzIgICAgID49IHBvb2xJbi50b2tlbnMoMSkuXzIgLSByZXdhcmRFbWl0dGVkICYmIC8vIHJld2FyZCB0b2tlbiBhbW91bnQgY29ycmVjdGx5IHJlZHVjZWQKICBwb29sT3V0LnRva2Vucy5zaXplICAgICAgICA9PSBwb29sSW4udG9rZW5zLnNpemUgICAgICAgICAgICAgICYmIC8vIGNhbm5vdCBpbmplY3QgbW9yZSB0b2tlbnMgdG8gcG9vbCBib3gKICBwb29sT3V0LlI0W0xvbmddLmdldCAgICAgPT0gYXZlcmFnZSAgICAgICAgICAgICAgICAgICAgICAgICAgICYmIC8vIHJhdGUKICBwb29sT3V0LlI1W0ludF0uZ2V0ICAgICAgPT0gcG9vbEluLlI1W0ludF0uZ2V0ICsgMSAgICAgICAgICAgICYmIC8vIGNvdW50ZXIKICBwb29sT3V0LnByb3Bvc2l0aW9uQnl0ZXMgPT0gcG9vbEluLnByb3Bvc2l0aW9uQnl0ZXMgICAgICAgICAgICYmIC8vIHByZXNlcnZlIHBvb2wgc2NyaXB0CiAgcG9vbE91dC52YWx1ZSAgICAgICAgICAgID49IHBvb2xJbi52YWx1ZSAgICAgICAgICAgICAgICAgICAgICAmJgogIHBvb2xPdXQuY3JlYXRpb25JbmZvLl8xICA+PSBIRUlHSFQgLSBidWZmZXIgICAgICAgICAgICAgICAgICAgJiYgLy8gZW5zdXJlIHRoYXQgbmV3IGJveCBoYXMgY29ycmVjdCBzdGFydCBlcG9jaCBoZWlnaHQKICBzZWxmT3V0LnRva2VucyAgICAgICAgICAgPT0gU0VMRi50b2tlbnMgICAgICAgICAgICAgICAgICAgICAgICYmIC8vIHJlZnJlc2ggTkZUIHByZXNlcnZlZAogIHNlbGZPdXQucHJvcG9zaXRpb25CeXRlcyA9PSBTRUxGLnByb3Bvc2l0aW9uQnl0ZXMgICAgICAgICAgICAgJiYgLy8gc2NyaXB0IHByZXNlcnZlZAogIHNlbGZPdXQudmFsdWUgICAgICAgICAgICA+PSBTRUxGLnZhbHVlICAgICAgICAgICAgICAgICAgICAgICAKfQ==_

    const P2S: &'static str = "oq3jWGvabYxVYtceq1RGzFD4UdcdHcqY861G7H4mDiEnYQHya17A2w5r7u45moTpjAqfsNTm2XyhRNvYHiZhDTpmnfVa9XHSsbs5zjEw5UmgQfuP5d3NdFVy7oiAvLP1sjZN8qiHryzFoenLgtsxV8wLAeBaRChy73dd3rgyVfZipVL5LCXQyXMqp9oFFzPtTPkBw3ha7gJ4Bs5KjeUkVXJRVQ2Tdhg51Sdb6fEkHRtRuvCpynxYokQXP6SNif1M6mPcBR3B4zMLcFvmGxwNkZ3mRFzqHVzHV8Syu5AzueJEmMTrvWAXnhpYE7WcFbmDt3dqyXq7x9DNyKq1VwRwgFscLYDenAHqqHKd3jsJ6Grs8uFvvvJGKdqzdoJ3qCcCRXeDcZAKmExJMH4hJbsk8b1ct5YDBcNrq3LUr319XkS8miZDbHdHa88MSpCJQJmE51hmWVAV1yXrpyxqXqAXXPpSaGCP38BwCv8hYFK37DyA4mQd5r7vF9vNo5DEXwQ5wA2EivwRtNqpKUxXtKuZWTNC7Pu7NmvEHSuJPnaoCUujCiPtLM4dR64u8Gp7X3Ujo3o9zuMc6npemx3hf8rQS18QXgKJLwfeSqVYkicbVcGZRHsPsGxwrf1Wixp45E8d5e97MsKTCuqSskPKaHUdQYW1JZ8djcr4dxg1qQN81m7u2q8dwW6AK32mwRSS3nj27jkjML6n6GBpNZk9AtB2uMx3CHo6pZSaxgeCXuu3amrdeYmbuSqHUNZHU";

    pub const POOL_NFT_INDEX: usize = 17;
    pub const ORACLE_TOKEN_ID_INDEX: usize = 3;

    // Note: contract sets both `minDataPoints` and `buffer` to 4. Changing values in the script, we
    // can confirm the following indices.
    pub const MIN_DATA_POINTS_INDEX: usize = 13;
    pub const BUFFER_INDEX: usize = 21;

    pub const MAX_DEVIATION_PERCENT_INDEX: usize = 15;
    pub const EPOCH_LENGTH_INDEX: usize = 0;

    pub fn new() -> Self {
        let encoder = AddressEncoder::new(NetworkPrefix::Mainnet);
        let addr = encoder.parse_address_from_str(Self::P2S).unwrap();
        let ergo_tree = addr.script().unwrap();

        Self::from_ergo_tree(ergo_tree).unwrap()
    }

    // TODO: switch to `TryFrom`
    pub fn from_ergo_tree(ergo_tree: ErgoTree) -> Result<Self, RefreshContractError> {
        dbg!(ergo_tree.get_constants().unwrap());

        if ergo_tree
            .get_constant(Self::POOL_NFT_INDEX)
            .map_err(|_| RefreshContractError::NoPoolNftId)?
            .ok_or(RefreshContractError::NoPoolNftId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(RefreshContractError::NoPoolNftId);
        }

        if ergo_tree
            .get_constant(Self::ORACLE_TOKEN_ID_INDEX)
            .map_err(|_| RefreshContractError::NoOracleTokenId)?
            .ok_or(RefreshContractError::NoOracleTokenId)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(RefreshContractError::NoOracleTokenId);
        }

        if ergo_tree
            .get_constant(Self::MIN_DATA_POINTS_INDEX)
            .map_err(|_| RefreshContractError::NoMinDataPoints)?
            .ok_or(RefreshContractError::NoMinDataPoints)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(RefreshContractError::NoMinDataPoints);
        }

        if ergo_tree
            .get_constant(Self::BUFFER_INDEX)
            .map_err(|_| RefreshContractError::NoBuffer)?
            .ok_or(RefreshContractError::NoBuffer)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(RefreshContractError::NoBuffer);
        }

        if ergo_tree
            .get_constant(Self::MAX_DEVIATION_PERCENT_INDEX)
            .map_err(|_| RefreshContractError::NoMaxDeviationPercent)?
            .ok_or(RefreshContractError::NoMaxDeviationPercent)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(RefreshContractError::NoMaxDeviationPercent);
        }

        if ergo_tree
            .get_constant(Self::EPOCH_LENGTH_INDEX)
            .map_err(|_| RefreshContractError::NoEpochLength)?
            .ok_or(RefreshContractError::NoEpochLength)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(RefreshContractError::NoEpochLength);
        }

        Ok(Self { ergo_tree })
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn epoch_length(&self) -> u32 {
        self.ergo_tree
            .get_constant(Self::EPOCH_LENGTH_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn with_epoch_length(self, epoch_length: u32) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::EPOCH_LENGTH_INDEX, (epoch_length as i32).into())
            .unwrap();
        Self { ergo_tree: tree }
    }

    pub fn buffer(&self) -> u32 {
        self.ergo_tree
            .get_constant(Self::BUFFER_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn with_buffer(self, buffer: u32) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::BUFFER_INDEX, (buffer as i32).into())
            .unwrap();
        Self { ergo_tree: tree }
    }

    pub fn min_data_points(&self) -> u32 {
        self.ergo_tree
            .get_constant(Self::MIN_DATA_POINTS_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn with_min_data_points(self, min_data_points: u32) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::MIN_DATA_POINTS_INDEX, (min_data_points as i32).into())
            .unwrap();
        Self { ergo_tree: tree }
    }

    pub fn max_deviation_percent(&self) -> u32 {
        self.ergo_tree
            .get_constant(Self::MAX_DEVIATION_PERCENT_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn with_max_deviation_percent(self, max_deviation_percent: u32) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(
                Self::MAX_DEVIATION_PERCENT_INDEX,
                (max_deviation_percent as i32).into(),
            )
            .unwrap();
        Self { ergo_tree: tree }
    }

    pub fn oracle_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(Self::ORACLE_TOKEN_ID_INDEX)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn with_oracle_token_id(self, token_id: TokenId) -> Self {
        let tree = self
            .ergo_tree
            .with_constant(Self::ORACLE_TOKEN_ID_INDEX, token_id.clone().into())
            .unwrap();
        Self { ergo_tree: tree }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_parsing() {
        let c = RefreshContract::new();
        assert_eq!(
            c.pool_nft_token_id(),
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap()
        );
        assert_eq!(
            c.oracle_token_id(),
            TokenId::from_base64("KkctSmFOZFJnVWtYcDJzNXY4eS9CP0UoSCtNYlBlU2g=").unwrap()
        );
        assert_eq!(c.min_data_points(), 4);
        assert_eq!(c.buffer(), 4);
        assert_eq!(c.max_deviation_percent(), 5);
        assert_eq!(c.epoch_length(), 30);
    }
}
