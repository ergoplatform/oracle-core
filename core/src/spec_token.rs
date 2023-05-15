//! Special tokens (newtypes) for tokens used in oracle_core
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenAmount;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SpecToken<T: TokenIdKind> {
    pub token_id: T,
    pub amount: TokenAmount,
}

impl<T: TokenIdKind> From<SpecToken<T>> for Token {
    fn from(spec_token: SpecToken<T>) -> Token {
        Token {
            token_id: spec_token.token_id.token_id(),
            amount: spec_token.amount,
        }
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
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct PoolTokenId(TokenId);

impl TokenIdKind for PoolTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct UpdateTokenId(TokenId);

impl TokenIdKind for UpdateTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct RefreshTokenId(TokenId);

impl TokenIdKind for RefreshTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct RewardTokenId(TokenId);
impl TokenIdKind for RewardTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct OracleTokenId(TokenId);

impl TokenIdKind for OracleTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct BallotTokenId(TokenId);
impl TokenIdKind for BallotTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct BuybackTokenId(TokenId);
impl TokenIdKind for BuybackTokenId {
    fn token_id(&self) -> TokenId {
        self.0
    }
    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self(token)
    }
}
