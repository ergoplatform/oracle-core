use derive_more::From;
use ergo_lib::ergotree_ir::chain::ergo_box::BoxId;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::sigma_protocol::dlog_group::EcPoint;
use thiserror::Error;

pub trait OracleBox {
    fn box_id(&self) -> BoxId;
    fn oracle_token(&self) -> Token;
    fn reward_token(&self) -> Token;
    fn public_key(&self) -> EcPoint;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> u64;
    fn get_box(&self) -> ErgoBox;
}

#[derive(Debug, From, Error)]
pub enum OracleBoxError {}

#[derive(Clone)]
pub struct OracleBoxWrapper(ErgoBox);

impl OracleBoxWrapper {
    pub fn new(b: ErgoBox) -> Result<Self, OracleBoxError> {
        todo!()
    }
}

impl OracleBox for OracleBoxWrapper {
    fn box_id(&self) -> BoxId {
        todo!()
    }

    fn oracle_token(&self) -> Token {
        todo!()
    }

    fn reward_token(&self) -> Token {
        todo!()
    }

    fn public_key(&self) -> EcPoint {
        todo!()
    }

    fn epoch_counter(&self) -> u32 {
        todo!()
    }

    fn rate(&self) -> u64 {
        todo!()
    }

    fn get_box(&self) -> ErgoBox {
        self.0.clone()
    }
}
