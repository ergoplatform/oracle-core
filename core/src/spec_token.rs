use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenAmount;
/// Special tokens (newtypes) for tokens used in oracle_core
use ergo_lib::ergotree_ir::chain::token::TokenId;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SpecToken<T: TokenIdKind> {
    pub token_id: T,
    pub amount: TokenAmount,
}

impl<T: TokenIdKind> Into<Token> for SpecToken<T> {
    fn into(self) -> Token {
        Token {
            token_id: self.token_id.token_id(),
            amount: self.amount,
        }
    }
}

impl<T: TokenIdKind> TryFrom<Token> for SpecToken<T> {
    type Error = String;
    fn try_from(token: Token) -> Result<Self, Self::Error> {
        let token_id = T::load_from_oracle_config()?;
        if token.token_id != token_id.token_id() {
            return Err("Token ID does not match ORACLE_CONFIG".into());
        }
        Ok(SpecToken {
            token_id,
            amount: token.amount,
        })
    }
}

impl<T: TokenIdKind> SpecToken<T> {
    pub fn token_id(&self) -> TokenId {
        self.token_id.token_id()
    }
    pub fn token_amount(&self) -> TokenAmount {
        self.amount
    }
}

pub trait TokenIdKind: Sized {
    fn token_id(&self) -> TokenId;
    /// Create a new TokenIdKind from a TokenId. Note that this does not validate the token id against the config file
    fn from_token_id_unchecked(token: TokenId) -> Self;
    /// Create a new TokenIdKind from ORACLE_CONFIG
    fn load_from_oracle_config() -> Result<Self, String>;
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct PoolTokenId(TokenId);

impl TokenIdKind for PoolTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
    fn load_from_oracle_config() -> Result<Self, String> {
        crate::oracle_config::MAYBE_ORACLE_CONFIG
            .as_ref()
            .map(|cfg| cfg.token_ids.pool_nft_token_id.clone())
            .map_err(|e| e.clone())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct UpdateTokenId(TokenId);

impl TokenIdKind for UpdateTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
    fn load_from_oracle_config() -> Result<Self, String> {
        crate::oracle_config::MAYBE_ORACLE_CONFIG
            .as_ref()
            .map(|cfg| cfg.token_ids.update_nft_token_id.clone())
            .map_err(|e| e.clone())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct RefreshTokenId(TokenId);

impl TokenIdKind for RefreshTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
    fn load_from_oracle_config() -> Result<Self, String> {
        crate::oracle_config::MAYBE_ORACLE_CONFIG
            .as_ref()
            .map(|cfg| cfg.token_ids.refresh_nft_token_id.clone())
            .map_err(|e| e.clone())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct RewardTokenId(TokenId);
impl TokenIdKind for RewardTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
    fn load_from_oracle_config() -> Result<Self, String> {
        crate::oracle_config::MAYBE_ORACLE_CONFIG
            .as_ref()
            .map(|cfg| cfg.token_ids.reward_token_id.clone())
            .map_err(|e| e.clone())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct OracleTokenId(TokenId);

impl TokenIdKind for OracleTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
    fn load_from_oracle_config() -> Result<Self, String> {
        crate::oracle_config::MAYBE_ORACLE_CONFIG
            .as_ref()
            .map(|cfg| cfg.token_ids.oracle_token_id.clone())
            .map_err(|e| e.clone())
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct BallotTokenId(TokenId);
impl TokenIdKind for BallotTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
    fn load_from_oracle_config() -> Result<Self, String> {
        crate::oracle_config::MAYBE_ORACLE_CONFIG
            .as_ref()
            .map(|cfg| cfg.token_ids.ballot_token_id.clone())
            .map_err(|e| e.clone())
    }
}
