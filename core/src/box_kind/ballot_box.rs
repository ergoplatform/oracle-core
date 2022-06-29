use crate::contracts::ballot::{BallotContract, BallotContractError};
use ergo_lib::{
    chain::ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
    ergo_chain_types::{Digest32, EcPoint},
    ergotree_ir::{
        chain::{
            ergo_box::{box_value::BoxValue, ErgoBox, ErgoBoxCandidate, NonMandatoryRegisterId},
            token::{Token, TokenId},
        },
        mir::constant::TryExtractInto,
        sigma_protocol::sigma_boolean::ProveDlog,
    },
};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BallotBoxError {
    #[error("ballot box: no ballot token found")]
    NoBallotToken,
    #[error("ballot box: no reward token id in R7 register")]
    NoRewardTokenIdInR7,
    #[error("ballot box: no reward token quantity in R8 register")]
    NoRewardTokenQuantityInR8,
    #[error("ballot box: no group element in R4 register")]
    NoGroupElementInR4,
    #[error("ballot box: no update box creation height in R5 register")]
    NoUpdateBoxCreationHeightInR5,
    #[error("ballot box: no pool box address hash in R6 register")]
    NoPoolBoxAddressInR6,
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
            .ok_or(BallotBoxError::NoBallotToken)?
            .get(0)
            .ok_or(BallotBoxError::NoBallotToken)?
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

        if ergo_box
            .get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(BallotBoxError::NoUpdateBoxCreationHeightInR5)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(BallotBoxError::NoUpdateBoxCreationHeightInR5);
        }

        if ergo_box
            .get_register(NonMandatoryRegisterId::R6.into())
            .ok_or(BallotBoxError::NoPoolBoxAddressInR6)?
            .try_extract_into::<Digest32>()
            .is_err()
        {
            return Err(BallotBoxError::NoPoolBoxAddressInR6);
        }

        if ergo_box
            .get_register(NonMandatoryRegisterId::R7.into())
            .ok_or(BallotBoxError::NoRewardTokenIdInR7)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(BallotBoxError::NoRewardTokenIdInR7);
        }

        if ergo_box
            .get_register(NonMandatoryRegisterId::R8.into())
            .ok_or(BallotBoxError::NoRewardTokenQuantityInR8)?
            .try_extract_into::<TokenId>()
            .is_err()
        {
            return Err(BallotBoxError::NoRewardTokenQuantityInR8);
        }

        let contract = BallotContract::from_ergo_tree(ergo_box.ergo_tree.clone())?;
        Ok(Self { ergo_box, contract })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_local_ballot_box_candidate(
    contract: &BallotContract,
    public_key: ProveDlog,
    update_box_creation_height: u32,
    ballot_token: Token,
    pool_box_address_hash: Digest32,
    reward_tokens: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, (*public_key.h).clone().into());
    builder.set_register_value(
        NonMandatoryRegisterId::R5,
        (update_box_creation_height as i32).into(),
    );
    builder.set_register_value(NonMandatoryRegisterId::R6, pool_box_address_hash.into());
    builder.set_register_value(
        NonMandatoryRegisterId::R7,
        reward_tokens.token_id.clone().into(),
    );
    builder.set_register_value(
        NonMandatoryRegisterId::R8,
        (*reward_tokens.amount.as_u64() as i32).into(),
    );
    builder.add_token(ballot_token);
    builder.build()
}
