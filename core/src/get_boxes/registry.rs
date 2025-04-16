use crate::pool_config::POOL_CONFIG;
use crate::spec_token::BallotTokenId;
use crate::spec_token::BuybackTokenId;
use crate::spec_token::OracleTokenId;
use crate::spec_token::PoolTokenId;
use crate::spec_token::RefreshTokenId;
use crate::spec_token::UpdateTokenId;

use super::generic_token_fetch::GenericTokenFetch;
use ::serde::Deserialize;
use ::serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenFetchRegistry {
    #[serde(rename = "All Datapoints Fetch")]
    pub oracle_token_fetch: GenericTokenFetch<OracleTokenId>,
    #[serde(rename = "Pool Box Fetch")]
    pub pool_token_fetch: GenericTokenFetch<PoolTokenId>,
    #[serde(rename = "Ballot Box Fetch")]
    pub ballot_token_fetch: GenericTokenFetch<BallotTokenId>,
    #[serde(rename = "Refresh Box Fetch")]
    pub refresh_token_fetch: GenericTokenFetch<RefreshTokenId>,
    #[serde(rename = "Update Box Fetch")]
    pub update_token_fetch: GenericTokenFetch<UpdateTokenId>,
    pub buyback_token_fetch: Option<GenericTokenFetch<BuybackTokenId>>,
}

impl TokenFetchRegistry {
    pub fn load() -> Result<Self, anyhow::Error> {
        log::info!("Registering token fetches");
        let pool_config = &POOL_CONFIG;
        let oracle_token_fetch =
            GenericTokenFetch::register(&pool_config.token_ids.oracle_token_id)?;
        let pool_token_fetch =
            GenericTokenFetch::register(&pool_config.token_ids.pool_nft_token_id)?;
        let ballot_token_fetch =
            GenericTokenFetch::register(&pool_config.token_ids.ballot_token_id)?;
        let refresh_token_fetch =
            GenericTokenFetch::register(&pool_config.token_ids.refresh_nft_token_id)?;
        let update_token_fetch =
            GenericTokenFetch::register(&pool_config.token_ids.update_nft_token_id)?;
        let buyback_token_fetch =
            if let Some(buyback_token_id) = pool_config.buyback_token_id.clone() {
                Some(GenericTokenFetch::register(&buyback_token_id)?)
            } else {
                None
            };
        let registry = Self {
            oracle_token_fetch,
            pool_token_fetch,
            ballot_token_fetch,
            refresh_token_fetch,
            update_token_fetch,
            buyback_token_fetch,
        };
        Ok(registry)
    }
}
