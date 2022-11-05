use ergo_lib::ergotree_ir::chain::token::TokenAmount;
/// Special tokens (newtypes) for tokens used in oracle_core
use ergo_lib::ergotree_ir::chain::token::TokenId;
use serde::Deserialize;
use serde::Serialize;

pub struct SpecToken<T: TokenIdKind> {
    id: T,
    amount: TokenAmount,
}

impl<T: TokenIdKind> SpecToken<T> {
    fn token_id(&self) -> TokenId {
        self.id.token_id()
    }
}

pub trait TokenIdKind {
    fn token_id(&self) -> TokenId;
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct PoolTokenId(TokenId);

impl TokenIdKind for PoolTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct UpdateTokenId(TokenId);

impl TokenIdKind for UpdateTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct RefreshTokenId(TokenId);

impl TokenIdKind for RefreshTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct RewardTokenId(TokenId);
impl TokenIdKind for RewardTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct OracleTokenId(TokenId);

impl TokenIdKind for OracleTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(transparent)]
pub struct BallotTokenId(TokenId);
impl TokenIdKind for BallotTokenId {
    fn token_id(&self) -> TokenId {
        self.0.clone()
    }
}
