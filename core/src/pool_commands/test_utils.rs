//! This module contains common code used for testing the various commands
use std::convert::TryFrom;
use std::convert::TryInto;

use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::TxId;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::Constant;
use ergo_lib::ergotree_ir::mir::expr::Expr;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use ergo_node_interface::node_interface::NodeError;
use sigma_test_util::force_any_val;

use crate::box_kind::OracleBoxWrapper;
use crate::box_kind::PoolBoxWrapper;
use crate::contracts::oracle::OracleContract;
use crate::contracts::pool::PoolContract;
use crate::oracle_state::{LocalDatapointBoxSource, PoolBoxSource, StageError};

use super::*;

#[derive(Clone)]
pub(crate) struct PoolBoxMock {
    pub pool_box: PoolBoxWrapper,
}

impl PoolBoxSource for PoolBoxMock {
    fn get_pool_box(&self) -> std::result::Result<PoolBoxWrapper, StageError> {
        Ok(self.pool_box.clone())
    }
}

#[derive(Clone)]
pub(crate) struct OracleBoxMock {
    pub oracle_box: OracleBoxWrapper,
}

impl LocalDatapointBoxSource for OracleBoxMock {
    fn get_local_oracle_datapoint_box(&self) -> std::result::Result<OracleBoxWrapper, StageError> {
        Ok(self.oracle_box.clone())
    }
}

#[derive(Clone)]
pub(crate) struct WalletDataMock {
    pub unspent_boxes: Vec<ErgoBox>,
}

impl WalletDataSource for WalletDataMock {
    fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, NodeError> {
        Ok(self.unspent_boxes.clone())
    }
}

pub(crate) fn make_pool_box(
    datapoint: i64,
    epoch_counter: i32,
    pool_nft_token_id: TokenId,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> PoolBoxWrapper {
    let tokens = vec![
        Token::from((pool_nft_token_id.clone(), 1u64.try_into().unwrap())),
        reward_token,
    ]
    .try_into()
    .unwrap();
    ErgoBox::new(
        value,
        PoolContract::new().ergo_tree(),
        Some(tokens),
        NonMandatoryRegisters::new(
            vec![
                (NonMandatoryRegisterId::R4, Constant::from(datapoint)),
                (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap(),
        creation_height,
        force_any_val::<TxId>(),
        0,
    )
    .unwrap()
    .try_into()
    .unwrap()
}

pub(crate) fn make_datapoint_box(
    pub_key: EcPoint,
    datapoint: i64,
    epoch_counter: i32,
    oracle_token_id: TokenId,
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
) -> ErgoBox {
    let tokens = vec![
        Token::from((oracle_token_id.clone(), 1u64.try_into().unwrap())),
        reward_token,
    ]
    .try_into()
    .unwrap();
    ErgoBox::new(
        value,
        OracleContract::new().ergo_tree(),
        Some(tokens),
        NonMandatoryRegisters::new(
            vec![
                (NonMandatoryRegisterId::R4, Constant::from(pub_key)),
                (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
                (NonMandatoryRegisterId::R6, Constant::from(datapoint)),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap(),
        creation_height,
        force_any_val::<TxId>(),
        0,
    )
    .unwrap()
}

pub(crate) fn make_wallet_unspent_box(pub_key: ProveDlog, value: BoxValue) -> ErgoBox {
    let c: Constant = pub_key.into();
    let expr: Expr = c.into();
    ErgoBox::new(
        value,
        ErgoTree::try_from(expr).unwrap(),
        None,
        NonMandatoryRegisters::empty(),
        1,
        force_any_val::<TxId>(),
        0,
    )
    .unwrap()
}

pub(crate) fn find_input_boxes(
    tx: UnsignedTransaction,
    available_boxes: Vec<ErgoBox>,
) -> Vec<ErgoBox> {
    tx.inputs
        .mapped(|i| {
            available_boxes
                .clone()
                .into_iter()
                .find(|b| b.box_id() == i.box_id)
                .unwrap()
        })
        .as_vec()
        .clone()
}
