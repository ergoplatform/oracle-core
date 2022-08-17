use crate::{
    contracts::ballot::{BallotContract, BallotContractError},
    oracle_config::{BallotBoxWrapperParameters, CastBallotBoxVoteParameters},
};
use ergo_lib::{
    chain::ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
    ergo_chain_types::{Digest32, EcPoint},
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoderError},
            ergo_box::{box_value::BoxValue, ErgoBox, ErgoBoxCandidate, NonMandatoryRegisterId},
            token::{Token, TokenId},
        },
        mir::constant::{TryExtractFromError, TryExtractInto},
        serialization::SigmaSerializationError,
        sigma_protocol::sigma_boolean::ProveDlog,
    },
};
use log::warn;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BallotBoxError {
    #[error("ballot box: no ballot token found")]
    NoBallotToken,
    #[error("ballot box: unknown ballot token id in `TOKENS(0)`")]
    UnknownBallotTokenId,
    #[error("ballot box: no reward token id in R7 register")]
    NoRewardTokenIdInR7,
    #[error("ballot box: no reward token quantity in R8 register")]
    NoRewardTokenQuantityInR8,
    #[error("ballot box: no group element in R4 register")]
    NoGroupElementInR4,
    #[error("ballot box: unexpected group element in R4 register")]
    UnexpectedGroupElementInR4,
    #[error("ballot box: no update box creation height in R5 register")]
    NoUpdateBoxCreationHeightInR5,
    #[error("ballot box: no pool box address hash in R6 register")]
    NoPoolBoxAddressInR6,
    #[error("ballot box: contract error {0:?}")]
    BallotContract(#[from] BallotContractError),
    #[error("ballot box: AddressEncoder error {0}")]
    AddressEncoder(#[from] AddressEncoderError),
    #[error("ballot box: TryExtractFrom error {0:?}")]
    TryExtractFrom(#[from] TryExtractFromError),
    #[error("ballot box: SigmaSerializationError {0:?}")]
    SigmaSerialization(#[from] SigmaSerializationError),
    #[error("ballot box: vote expected to be already cast, but hasn't")]
    ExpectedVoteCast,
}

pub trait BallotBox {
    fn contract(&self) -> &BallotContract;
    fn ballot_token(&self) -> Token;
    fn min_storage_rent(&self) -> u64;
    fn ballot_token_owner(&self) -> ProveDlog;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Clone, Debug)]
pub struct BallotBoxWrapper {
    ergo_box: ErgoBox,
    contract: BallotContract,
}

impl BallotBoxWrapper {
    pub fn new(ergo_box: ErgoBox, inputs: BallotBoxWrapperInputs) -> Result<Self, BallotBoxError> {
        let ballot_token_id = &ergo_box
            .tokens
            .as_ref()
            .ok_or(BallotBoxError::NoBallotToken)?
            .get(0)
            .ok_or(BallotBoxError::NoBallotToken)?
            .token_id;
        if *ballot_token_id != *inputs.ballot_token_id {
            return Err(BallotBoxError::UnknownBallotTokenId);
        }

        let ec = ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(BallotBoxError::NoGroupElementInR4)?
            .try_extract_into::<EcPoint>()?;
        if inputs.parameters.ballot_token_owner_address.address()
            != Address::P2Pk(ProveDlog::from(ec))
        {
            return Err(BallotBoxError::UnexpectedGroupElementInR4);
        }

        if let Some(CastBallotBoxVoteParameters {
            reward_token_id,
            reward_token_quantity,
            pool_box_address_hash,
            update_box_creation_height,
        }) = inputs.parameters.vote_parameters.as_ref()
        {
            let register_update_box_creation_height = ergo_box
                .get_register(NonMandatoryRegisterId::R5.into())
                .ok_or(BallotBoxError::NoUpdateBoxCreationHeightInR5)?
                .try_extract_into::<i32>()?;

            if register_update_box_creation_height != *update_box_creation_height {
                warn!("Update box creation height in R5 register differs to config. Could be due to vote.");
            }

            let register_pool_box_address_hash = ergo_box
                .get_register(NonMandatoryRegisterId::R6.into())
                .ok_or(BallotBoxError::NoPoolBoxAddressInR6)?
                .try_extract_into::<Digest32>()?;

            if *pool_box_address_hash != register_pool_box_address_hash {
                warn!("Pool box address in R6 register differs to config. Could be due to vote.");
            }

            let register_reward_token_id = ergo_box
                .get_register(NonMandatoryRegisterId::R7.into())
                .ok_or(BallotBoxError::NoRewardTokenIdInR7)?
                .try_extract_into::<TokenId>()?;

            if register_reward_token_id != *reward_token_id {
                warn!("Reward token id in R7 register differs to config. Could be due to vote.");
            }

            let register_reward_token_quantity: u64 = ergo_box
                .get_register(NonMandatoryRegisterId::R8.into())
                .ok_or(BallotBoxError::NoRewardTokenQuantityInR8)?
                .try_extract_into::<i64>()?
                as u64;

            if register_reward_token_quantity != *reward_token_quantity {
                warn!(
                    "Reward token quantity in R8 register differs to config. Could be due to vote."
                );
            }
        } else if ergo_box.additional_registers.len() > 1 {
            // If no vote parameter is provided, then the box must only have R4 (owner public key) defined
            return Err(BallotBoxError::ExpectedVoteCast);
        }

        let contract = BallotContract::from_ergo_tree(ergo_box.ergo_tree.clone(), inputs.into())?;
        Ok(Self { ergo_box, contract })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BallotBoxWrapperInputs<'a> {
    pub parameters: &'a BallotBoxWrapperParameters,
    /// Ballot token is expected to reside in `tokens(0)` of the ballot box.
    pub ballot_token_id: &'a TokenId,
    /// This token id appears as a constant in the ballot contract.
    pub update_nft_token_id: &'a TokenId,
}

/// A Ballot Box with vote parameters guaranteed to be set
#[derive(Clone, Debug)]
pub struct VoteBallotBoxWrapper {
    ergo_box: ErgoBox,
    vote_parameters: CastBallotBoxVoteParameters,
    contract: BallotContract,
}

impl VoteBallotBoxWrapper {
    pub fn new(ergo_box: ErgoBox, inputs: BallotBoxWrapperInputs) -> Result<Self, BallotBoxError> {
        let ballot_token_id = &ergo_box
            .tokens
            .as_ref()
            .ok_or(BallotBoxError::NoBallotToken)?
            .get(0)
            .ok_or(BallotBoxError::NoBallotToken)?
            .token_id;
        if *ballot_token_id != *inputs.ballot_token_id {
            return Err(BallotBoxError::UnknownBallotTokenId);
        }

        if ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(BallotBoxError::NoGroupElementInR4)?
            .try_extract_into::<EcPoint>()
            .is_err()
        {
            return Err(BallotBoxError::NoGroupElementInR4);
        }
        let update_box_creation_height = ergo_box
            .get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(BallotBoxError::NoUpdateBoxCreationHeightInR5)?
            .try_extract_into::<i32>()?;

        let pool_box_address_hash = ergo_box
            .get_register(NonMandatoryRegisterId::R6.into())
            .ok_or(BallotBoxError::NoPoolBoxAddressInR6)?
            .try_extract_into::<Digest32>()?;

        let reward_token_id = ergo_box
            .get_register(NonMandatoryRegisterId::R7.into())
            .ok_or(BallotBoxError::NoRewardTokenIdInR7)?
            .try_extract_into::<TokenId>()?;
        let reward_token_quantity = ergo_box
            .get_register(NonMandatoryRegisterId::R8.into())
            .ok_or(BallotBoxError::NoRewardTokenQuantityInR8)?
            .try_extract_into::<i64>()? as u64;

        let contract = BallotContract::from_ergo_tree(ergo_box.ergo_tree.clone(), inputs.into())?;
        let vote_parameters = CastBallotBoxVoteParameters {
            pool_box_address_hash,
            reward_token_id,
            reward_token_quantity,
            update_box_creation_height,
        };
        Ok(Self {
            ergo_box,
            contract,
            vote_parameters,
        })
    }

    pub fn vote_parameters(&self) -> &CastBallotBoxVoteParameters {
        &self.vote_parameters
    }
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

    fn ballot_token_owner(&self) -> ProveDlog {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<EcPoint>()
            .unwrap()
            .into()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }
}

impl BallotBox for VoteBallotBoxWrapper {
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

    fn ballot_token_owner(&self) -> ProveDlog {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<EcPoint>()
            .unwrap()
            .into()
    }

    fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_local_ballot_box_candidate(
    contract: &BallotContract,
    ballot_token_owner: ProveDlog,
    update_box_creation_height: u32,
    ballot_token: Token,
    pool_box_address_hash: Digest32,
    reward_tokens: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(
        NonMandatoryRegisterId::R4,
        (*ballot_token_owner.h).clone().into(),
    );
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
        (*reward_tokens.amount.as_u64() as i64).into(),
    );
    builder.add_token(ballot_token);
    builder.build()
}
