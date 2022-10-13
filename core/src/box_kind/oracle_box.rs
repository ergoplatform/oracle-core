use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use thiserror::Error;

use crate::contracts::oracle::OracleContract;
use crate::contracts::oracle::OracleContractError;
use crate::contracts::oracle::OracleContractInputs;
use crate::contracts::oracle::OracleContractParameters;

pub trait OracleBox {
    fn contract(&self) -> &OracleContract;
    fn oracle_token(&self) -> Token;
    fn reward_token(&self) -> Token;
    fn public_key(&self) -> ProveDlog;
    fn get_box(&self) -> &ErgoBox;
}

#[derive(Debug, Error)]
pub enum OracleBoxError {
    #[error("oracle box: no tokens found")]
    NoTokens,
    #[error("oracle box: no oracle token found")]
    NoOracleToken,
    #[error("oracle box: unknown oracle token id in `TOKENS(0)`")]
    UnknownOracleTokenId,
    #[error("oracle box: no reward token found")]
    NoRewardToken,
    #[error("oracle box: unknown reward token id in `TOKENS(1)`")]
    UnknownRewardTokenId,
    #[error("oracle box: no public key in R4")]
    NoPublicKeyInR4,
    #[error("oracle box: no epoch counter in R5")]
    NoEpochCounter,
    #[error("oracle box: no data point in R6")]
    NoDataPoint,
    #[error("oracle box: {0:?}")]
    OracleContractError(#[from] OracleContractError),
    #[error("oracle box: TryExtractFrom error {0:?}")]
    TryExtractFrom(#[from] TryExtractFromError),
    #[error("oracle box: Can't create EcPoint from String {0}")]
    EcPoint(String),
    #[error("oracle box: expected posted oracle box")]
    ExpectedPostedOracleBox,
}

#[derive(Clone)]
pub struct PostedOracleBox {
    ergo_box: ErgoBox,
    contract: OracleContract,
}

#[derive(Clone)]
pub struct CollectedOracleBox {
    ergo_box: ErgoBox,
    contract: OracleContract,
}

#[derive(Clone)]
pub enum OracleBoxWrapper {
    Posted(PostedOracleBox),
    Collected(CollectedOracleBox),
}

impl OracleBoxWrapper {
    pub fn new(b: ErgoBox, inputs: &OracleBoxWrapperInputs) -> Result<Self, OracleBoxError> {
        let oracle_token_id = b
            .tokens
            .as_ref()
            .ok_or(OracleBoxError::NoTokens)?
            .first()
            .token_id
            .clone();

        if oracle_token_id != inputs.oracle_token_id {
            return Err(OracleBoxError::UnknownOracleTokenId);
        }

        let reward_token_id = b
            .tokens
            .as_ref()
            .ok_or(OracleBoxError::NoTokens)?
            .get(1)
            .ok_or(OracleBoxError::NoRewardToken)?
            .token_id
            .clone();

        if reward_token_id != inputs.reward_token_id {
            return Err(OracleBoxError::UnknownRewardTokenId);
        }

        // We won't be analysing the actual address since there exists multiple oracle boxes that
        // will be inputs for the 'refresh pool' operation.
        let _ = b
            .get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(OracleBoxError::NoPublicKeyInR4)?
            .try_extract_into::<EcPoint>()?;

        let epoch_counter_opt = b
            .get_register(NonMandatoryRegisterId::R5.into())
            .and_then(|r| r.try_extract_into::<i32>().ok());

        let rate_opt = b
            .get_register(NonMandatoryRegisterId::R6.into())
            .and_then(|r| r.try_extract_into::<i64>().ok());

        let contract =
            OracleContract::from_ergo_tree(b.ergo_tree.clone(), &inputs.contract_inputs)?;

        let collected_oracle_box = OracleBoxWrapper::Collected(CollectedOracleBox {
            ergo_box: b.clone(),
            contract: contract.clone(),
        });

        let posted_oracle_box = OracleBoxWrapper::Posted(PostedOracleBox {
            ergo_box: b,
            contract,
        });

        match (epoch_counter_opt, rate_opt) {
            (Some(_), Some(_)) => Ok(posted_oracle_box),
            (None, None) => Ok(collected_oracle_box),
            (Some(_), None) => Err(OracleBoxError::NoDataPoint),
            (None, Some(_)) => Err(OracleBoxError::NoEpochCounter),
        }
    }
}

impl OracleBox for OracleBoxWrapper {
    fn oracle_token(&self) -> Token {
        self.get_box()
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
    }

    fn reward_token(&self) -> Token {
        self.get_box()
            .tokens
            .as_ref()
            .unwrap()
            .get(1)
            .unwrap()
            .clone()
    }

    fn public_key(&self) -> ProveDlog {
        self.get_box()
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<EcPoint>()
            .unwrap()
            .into()
    }

    fn get_box(&self) -> &ErgoBox {
        match self {
            OracleBoxWrapper::Posted(p) => &p.ergo_box,
            OracleBoxWrapper::Collected(c) => &c.ergo_box,
        }
    }

    fn contract(&self) -> &OracleContract {
        match self {
            OracleBoxWrapper::Posted(p) => &p.contract,
            OracleBoxWrapper::Collected(c) => &c.contract,
        }
    }
}

impl PostedOracleBox {
    pub fn new(b: ErgoBox, inputs: &OracleBoxWrapperInputs) -> Result<Self, OracleBoxError> {
        OracleBoxWrapper::new(b, inputs).and_then(|b| match b {
            OracleBoxWrapper::Posted(p) => Ok(p),
            OracleBoxWrapper::Collected(_) => Err(OracleBoxError::ExpectedPostedOracleBox),
        })
    }

    pub fn oracle_token(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .clone()
    }

    pub fn reward_token(&self) -> Token {
        self.ergo_box
            .tokens
            .as_ref()
            .unwrap()
            .get(1)
            .unwrap()
            .clone()
    }

    pub fn public_key(&self) -> ProveDlog {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<EcPoint>()
            .unwrap()
            .into()
    }

    pub fn contract(&self) -> &OracleContract {
        &self.contract
    }

    pub fn get_box(&self) -> &ErgoBox {
        &self.ergo_box
    }

    pub fn epoch_counter(&self) -> u32 {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R5.into())
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    pub fn rate(&self) -> u64 {
        self.ergo_box
            .get_register(NonMandatoryRegisterId::R6.into())
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }
}

#[derive(Clone, Debug)]
pub struct OracleBoxWrapperInputs {
    pub contract_inputs: OracleContractInputs,
    /// Ballot token is expected to reside in `tokens(0)` of the oracle box.
    pub oracle_token_id: TokenId,
    /// Reward token is expected to reside in `tokens(1)` of the oracle box.
    pub reward_token_id: TokenId,
}

impl OracleBoxWrapperInputs {
    pub fn checked_load(
        oracle_contract_parameters: OracleContractParameters,
        pool_token_id: TokenId,
        oracle_token_id: TokenId,
        reward_token_id: TokenId,
    ) -> Result<Self, OracleContractError> {
        let contract_inputs =
            OracleContractInputs::checked_load(oracle_contract_parameters, pool_token_id)?;
        Ok(Self {
            contract_inputs,
            oracle_token_id,
            reward_token_id,
        })
    }

    pub fn build_with(
        oracle_contract_parameters: OracleContractParameters,
        pool_token_id: TokenId,
        oracle_token_id: TokenId,
        reward_token_id: TokenId,
    ) -> Result<Self, OracleContractError> {
        let contract_inputs =
            OracleContractInputs::build_with(oracle_contract_parameters, pool_token_id)?;
        Ok(Self {
            contract_inputs,
            oracle_token_id,
            reward_token_id,
        })
    }
}

impl From<OracleBoxWrapper> for ErgoBox {
    fn from(w: OracleBoxWrapper) -> Self {
        w.get_box().clone()
    }
}

impl From<PostedOracleBox> for ErgoBox {
    fn from(w: PostedOracleBox) -> Self {
        w.ergo_box.clone()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_oracle_box_candidate(
    contract: &OracleContract,
    public_key: ProveDlog,
    datapoint: i64,
    epoch_counter: u32,
    oracle_token: Token,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, (*public_key.h).clone().into());
    builder.set_register_value(NonMandatoryRegisterId::R5, (epoch_counter as i32).into());
    builder.set_register_value(NonMandatoryRegisterId::R6, (datapoint as i64).into());
    builder.add_token(oracle_token.clone());
    builder.add_token(reward_token.clone());
    builder.build()
}

/// Make an ergo box candidate to be an output box on data point colection (refresh action)
/// Without data point and epoch counter to prevent it to be used as input on next collection
pub fn make_collected_oracle_box_candidate(
    contract: &OracleContract,
    public_key: ProveDlog,
    oracle_token: Token,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, ErgoBoxCandidateBuilderError> {
    let mut builder = ErgoBoxCandidateBuilder::new(value, contract.ergo_tree(), creation_height);
    builder.set_register_value(NonMandatoryRegisterId::R4, (*public_key.h).clone().into());
    builder.add_token(oracle_token.clone());
    builder.add_token(reward_token.clone());
    builder.build()
}
