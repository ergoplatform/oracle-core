use crate::contracts::ballot::{BallotContract, BallotContractError};
use ergo_lib::{
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::{
            ergo_box::{ErgoBox, NonMandatoryRegisterId},
            token::Token,
        },
        mir::constant::TryExtractInto,
    },
};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BallotBoxError {
    #[error("ballot box: no tokens found")]
    NoTokens,
    #[error("ballot box: no group element in R4 register")]
    NoGroupElementInR4,
    #[error("ballot box: contract error {0:?}")]
    BallotContract(#[from] BallotContractError),
}

pub trait BallotBox {
    fn contract(&self) -> &BallotContract;
    fn ballot_token(&self) -> Token;
    fn min_storage_rent(&self) -> u64;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Clone)]
pub struct BallotBoxWrapper {
    ergo_box: ErgoBox,
    contract: BallotContract,
}

impl BallotBox for BallotBoxWrapper {
    fn contract(&self) -> &BallotContract {
        &self.contract
    }

    fn ballot_token(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
    }

    fn min_storage_rent(&self) -> u64 {
        self.contract.min_storage_rent()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }
}

impl TryFrom<ErgoBox> for BallotBoxWrapper {
    type Error = BallotBoxError;

    fn try_from(ergo_box: ErgoBox) -> Result<Self, Self::Error> {
        let _ballot_token_id = ergo_box
            .tokens
            .as_ref()
            .ok_or(BallotBoxError::NoTokens)?
            .get(0)
            .ok_or(BallotBoxError::NoTokens)?
            .token_id
            .clone();

        if ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(BallotBoxError::NoGroupElementInR4)?
            .try_extract_into::<EcPoint>()
            .is_err()
        {
            return Err(BallotBoxError::NoGroupElementInR4);
        }

        let contract = BallotContract::from_ergo_tree(ergo_box.ergo_tree.clone())?;
        Ok(Self { ergo_box, contract })
    }
}
