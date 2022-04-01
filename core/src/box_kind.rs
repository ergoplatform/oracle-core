use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::BoxId;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::sigma_protocol::dlog_group::EcPoint;
use thiserror::Error;

use crate::contracts::refresh::RefreshContract;

pub trait OracleBox {
    fn box_id(&self) -> BoxId;
    fn value(&self) -> BoxValue;
    fn ergo_tree(&self) -> ErgoTree;
    fn oracle_token(&self) -> Token;
    fn reward_token(&self) -> Token;
    fn public_key(&self) -> EcPoint;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> u64;
    fn get_box(&self) -> ErgoBox;
}

#[derive(Debug, Error)]
pub enum OracleBoxError {
    #[error("oracle box: incorrect oracle token id: {0:?}")]
    IncorrectOracleTokenId(TokenId),
    #[error("oracle box: incorrect reward token id: {0:?}")]
    IncorrectRewardTokenId(TokenId),
    #[error("oracle box: no tokens found")]
    NoTokens,
    #[error("oracle box: no reward token found")]
    NoRewardToken,
    #[error("oracle box: no public key in R4")]
    NoPublicKey,
    #[error("oracle box: no epoch counter in R5")]
    NoEpochCounter,
    #[error("oracle box: no data point in R6")]
    NoDataPoint,
}

#[derive(Clone)]
pub struct OracleBoxWrapper(ErgoBox);

impl OracleBoxWrapper {
    pub fn new(b: ErgoBox) -> Result<Self, OracleBoxError> {
        let refresh_contract = RefreshContract::new();
        let oracle_token_id = b
            .tokens
            .as_ref()
            .ok_or(OracleBoxError::NoTokens)?
            .get(0)
            .ok_or(OracleBoxError::NoTokens)?
            .token_id
            .clone();
        if oracle_token_id != refresh_contract.oracle_nft_token_id() {
            return Err(OracleBoxError::IncorrectOracleTokenId(oracle_token_id));
        }
        let reward_token_id = b
            .tokens
            .as_ref()
            .ok_or(OracleBoxError::NoTokens)?
            .get(1)
            .ok_or(OracleBoxError::NoRewardToken)?
            .token_id
            .clone();
        if reward_token_id
            != TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap()
        {
            return Err(OracleBoxError::IncorrectOracleTokenId(reward_token_id));
        }

        if b.get_register(NonMandatoryRegisterId::R4.into())
            .ok_or(OracleBoxError::NoPublicKey)?
            .try_extract_into::<EcPoint>()
            .is_err()
        {
            return Err(OracleBoxError::NoPublicKey);
        }

        if b.get_register(NonMandatoryRegisterId::R5.into())
            .ok_or(OracleBoxError::NoEpochCounter)?
            .try_extract_into::<i32>()
            .is_err()
        {
            return Err(OracleBoxError::NoEpochCounter);
        }

        if b.get_register(NonMandatoryRegisterId::R6.into())
            .ok_or(OracleBoxError::NoDataPoint)?
            .try_extract_into::<i64>()
            .is_err()
        {
            return Err(OracleBoxError::NoDataPoint);
        }

        Ok(Self(b))
    }
}

impl OracleBox for OracleBoxWrapper {
    fn box_id(&self) -> BoxId {
        self.0.box_id()
    }

    fn value(&self) -> BoxValue {
        self.0.value
    }

    fn ergo_tree(&self) -> ErgoTree {
        self.0.ergo_tree.clone()
    }

    fn oracle_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(0).unwrap().clone()
    }

    fn reward_token(&self) -> Token {
        self.0.tokens.as_ref().unwrap().get(1).unwrap().clone()
    }

    fn public_key(&self) -> EcPoint {
        self.0
            .get_register(NonMandatoryRegisterId::R4.into())
            .unwrap()
            .try_extract_into::<EcPoint>()
            .unwrap()
    }

    fn epoch_counter(&self) -> u32 {
        self.0
            .get_register(NonMandatoryRegisterId::R5.into())
            .unwrap()
            .try_extract_into::<i32>()
            .unwrap() as u32
    }

    fn rate(&self) -> u64 {
        self.0
            .get_register(NonMandatoryRegisterId::R6.into())
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }

    fn get_box(&self) -> ErgoBox {
        self.0.clone()
    }
}
