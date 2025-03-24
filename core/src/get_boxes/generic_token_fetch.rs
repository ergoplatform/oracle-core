use crate::spec_token::TokenIdKind;
use derive_more::From;
use derive_more::Into;
use ergo_lib::ergo_chain_types::Digest32;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use serde::Deserialize;
use serde::Serialize;

use super::GetBoxes;
use super::GetBoxesError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, From, Into)]
#[serde(try_from = "String", into = "String")]
pub struct GenericTokenFetch<T: TokenIdKind + std::clone::Clone> {
    token_id: TokenId,
    fantom: std::marker::PhantomData<T>,
}

impl<T: TokenIdKind + Clone> GenericTokenFetch<T> {
    pub fn new(token_id: TokenId) -> Self {
        Self {
            token_id,
            fantom: std::marker::PhantomData,
        }
    }

    pub fn register(token_id: &T) -> Result<Self, GetBoxesError> {
        let token_id = token_id.token_id();
        Ok(GenericTokenFetch::<T> {
            token_id,
            fantom: std::marker::PhantomData,
        })
    }
}

impl<T: TokenIdKind + Clone> TryFrom<String> for GenericTokenFetch<T> {
    type Error = GetBoxesError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let token_id = Digest32::try_from(value).unwrap().into();
        Ok(GenericTokenFetch {
            token_id,
            fantom: std::marker::PhantomData,
        })
    }
}

impl<T: TokenIdKind + Clone> From<GenericTokenFetch<T>> for String {
    fn from(token_fetch: GenericTokenFetch<T>) -> Self {
        token_fetch.token_id.into()
    }
}

impl<T: TokenIdKind + Clone> TokenIdKind for GenericTokenFetch<T> {
    fn token_id(&self) -> TokenId {
        self.token_id
    }

    fn from_token_id_unchecked(token: TokenId) -> Self {
        Self::new(token)
    }
}

impl<T: TokenIdKind + Clone> GetBoxes for GenericTokenFetch<T> {}
