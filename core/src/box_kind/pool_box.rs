use std::convert::TryFrom;

use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::Token;
use thiserror::Error;

pub trait PoolBox {
    fn pool_token(&self) -> Token;
    fn epoch_counter(&self) -> u32;
    fn rate(&self) -> u64;
    fn get_box(&self) -> ErgoBox;
}

#[derive(Debug, Error)]
pub enum PoolBoxError {}

#[derive(Clone)]
pub struct PoolBoxWrapper(ErgoBox);

impl PoolBox for PoolBoxWrapper {
    fn pool_token(&self) -> Token {
        todo!()
    }

    fn epoch_counter(&self) -> u32 {
        todo!()
    }

    fn rate(&self) -> u64 {
        todo!()
    }

    fn get_box(&self) -> ErgoBox {
        todo!()
    }
}

impl TryFrom<ErgoBox> for PoolBoxWrapper {
    type Error = PoolBoxError;

    fn try_from(value: ErgoBox) -> Result<Self, Self::Error> {
        todo!()
    }
}
