use crate::{
    contracts::ballot::{
        BallotContract, BallotContractError, BallotContractInputs, BallotContractParameters,
    },
    pool_config::CastBallotBoxVoteParameters,
    spec_token::{BallotTokenId, SpecToken, TokenIdKind, UpdateTokenId},
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
}

pub trait BallotBox {
    fn contract(&self) -> &BallotContract;
    fn ballot_token(&self) -> SpecToken<BallotTokenId>;
    fn min_storage_rent(&self) -> BoxValue;
    fn ballot_token_owner(&self) -> ProveDlog;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Clone, Debug)]
pub struct BallotBoxWrapper {
    ergo_box: ErgoBox,
    contract: BallotContract,
}

impl BallotBoxWrapper {
    pub fn new(
        ergo_box: ErgoBox,
        inputs: &BallotBoxWrapperInputs,
        ballot_token_owner_address: &Address,
    ) -> Result<Self, BallotBoxError> {
        let ballot_token_id = &ergo_box
            .tokens
            .as_ref()
            .ok_or(BallotBoxError::NoBallotToken)?
            .get(0)
            .ok_or(BallotBoxError::NoBallotToken)?
            .token_id;
        if *ballot_token_id != inputs.ballot_token_id.token_id() {
            return Err(BallotBoxError::UnknownBallotTokenId);
        }

        let ec = ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(BallotBoxError::NoGroupElementInR4)?
            .try_extract_into::<EcPoint>()?;
        if ballot_token_owner_address != &Address::P2Pk(ProveDlog::from(ec)) {
            return Err(BallotBoxError::UnexpectedGroupElementInR4);
        }

        let contract =
            BallotContract::from_ergo_tree(ergo_box.ergo_tree.clone(), &inputs.contract_inputs)?;
        Ok(Self { ergo_box, contract })
    }
}

#[derive(Clone, Debug)]
pub struct BallotBoxWrapperInputs {
    pub contract_inputs: BallotContractInputs,
    /// Ballot token is expected to reside in `tokens(0)` of the ballot box.
    pub ballot_token_id: BallotTokenId,
}

impl BallotBoxWrapperInputs {
    pub fn build_with(
        ballot_contract_parameters: BallotContractParameters,
        ballot_token_id: BallotTokenId,
        update_nft_token_id: UpdateTokenId,
    ) -> Result<Self, BallotContractError> {
        let contract_inputs =
            BallotContractInputs::build_with(ballot_contract_parameters, update_nft_token_id)?;
        Ok(BallotBoxWrapperInputs {
            contract_inputs,
            ballot_token_id,
        })
    }

    pub fn checked_load(
        ballot_contract_parameters: BallotContractParameters,
        ballot_token_id: BallotTokenId,
        update_nft_token_id: UpdateTokenId,
    ) -> Result<Self, BallotContractError> {
        let contract_inputs =
            BallotContractInputs::checked_load(ballot_contract_parameters, update_nft_token_id)?;
        Ok(BallotBoxWrapperInputs {
            contract_inputs,
            ballot_token_id,
        })
    }
}

/// A Ballot Box with vote parameters guaranteed to be set
#[derive(Clone, Debug)]
pub struct VoteBallotBoxWrapper {
    ergo_box: ErgoBox,
    vote_parameters: CastBallotBoxVoteParameters,
    contract: BallotContract,
}

impl VoteBallotBoxWrapper {
    pub fn new(ergo_box: ErgoBox, inputs: &BallotBoxWrapperInputs) -> Result<Self, BallotBoxError> {
        let ballot_token_id = &ergo_box
            .tokens
            .as_ref()
            .ok_or(BallotBoxError::NoBallotToken)?
            .get(0)
            .ok_or(BallotBoxError::NoBallotToken)?
            .token_id;
        if *ballot_token_id != inputs.ballot_token_id.token_id() {
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

        let contract =
            BallotContract::from_ergo_tree(ergo_box.ergo_tree.clone(), &inputs.contract_inputs)?;
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

    fn ballot_token(&self) -> SpecToken<BallotTokenId> {
        let ballot_token = self.ergo_box.tokens.as_ref().unwrap().get(0).unwrap();
        SpecToken {
            // Safe to do this here since BallotBoxWrapper::new() already checks token id
            token_id: BallotTokenId::from_token_id_unchecked(ballot_token.token_id.clone()),
            amount: ballot_token.amount,
        }
    }

    fn min_storage_rent(&self) -> BoxValue {
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

    fn ballot_token(&self) -> SpecToken<BallotTokenId> {
        let ballot_token = self.ergo_box.tokens.as_ref().unwrap().get(0).unwrap();
        SpecToken {
            // Safe to do this here since BallotBoxWrapper::new() already checks token id
            token_id: BallotTokenId::from_token_id_unchecked(ballot_token.token_id.clone()),
            amount: ballot_token.amount,
        }
    }

    fn min_storage_rent(&self) -> BoxValue {
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
    ballot_token: SpecToken<BallotTokenId>,
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
    builder.set_register_value(NonMandatoryRegisterId::R7, reward_tokens.token_id.into());
    builder.set_register_value(
        NonMandatoryRegisterId::R8,
        (*reward_tokens.amount.as_u64() as i64).into(),
    );
    builder.add_token(ballot_token.into());
    builder.build()
}
