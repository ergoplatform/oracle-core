//! This module contains common code used for testing the various commands
use std::convert::TryFrom;
use std::convert::TryInto;

use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::TxId;
use ergo_lib::chain::transaction::TxIoVec;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::BoxTokens;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::constant::Constant;
use ergo_lib::ergotree_ir::mir::expr::Expr;
use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
use ergo_lib::wallet::signing::TransactionContext;
use ergo_lib::wallet::Wallet;
use ergo_node_interface::node_interface::NodeError;
use sigma_test_util::force_any_val;

use crate::box_kind::BallotBoxWrapper;
use crate::box_kind::OracleBoxWrapper;
use crate::box_kind::PoolBoxWrapper;
use crate::contracts::oracle::OracleContract;
use crate::contracts::pool::PoolContract;
use crate::contracts::pool::PoolContractParameters;
use crate::contracts::refresh::RefreshContractParameters;
use crate::node_interface::SignTransaction;
use crate::oracle_state::LocalBallotBoxSource;
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
pub(crate) struct BallotBoxMock {
    pub ballot_box: BallotBoxWrapper,
}

impl LocalBallotBoxSource for BallotBoxMock {
    fn get_ballot_box(&self) -> std::result::Result<BallotBoxWrapper, StageError> {
        Ok(self.ballot_box.clone())
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
    reward_token: Token,
    value: BoxValue,
    creation_height: u32,
    pool_contract_parameters: &PoolContractParameters,
    oracle_contract_parameters: &OracleContractParameters,
) -> PoolBoxWrapper {
    let tokens = vec![
        Token::from((
            oracle_contract_parameters.pool_nft_token_id.clone(),
            1u64.try_into().unwrap(),
        )),
        reward_token,
    ]
    .try_into()
    .unwrap();
    (
        ErgoBox::new(
            value,
            PoolContract::new(pool_contract_parameters)
                .unwrap()
                .ergo_tree(),
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
        .unwrap(),
        pool_contract_parameters,
        oracle_contract_parameters,
    )
        .try_into()
        .unwrap()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn make_datapoint_box(
    pub_key: EcPoint,
    datapoint: i64,
    epoch_counter: i32,
    oracle_token_id: TokenId,
    pool_nft_token_id: TokenId,
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
    let mut parameters = make_oracle_contract_parameters();
    parameters.pool_nft_token_id = pool_nft_token_id;
    ErgoBox::new(
        value,
        OracleContract::new(&parameters).unwrap().ergo_tree(),
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

pub(crate) fn make_wallet_unspent_box(
    pub_key: ProveDlog,
    value: BoxValue,
    tokens: Option<BoxTokens>,
) -> ErgoBox {
    let c: Constant = pub_key.into();
    let expr: Expr = c.into();
    ErgoBox::new(
        value,
        ErgoTree::try_from(expr).unwrap(),
        tokens,
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

pub struct LocalTxSigner<'a> {
    pub ctx: &'a ErgoStateContext,
    pub wallet: &'a Wallet,
}

impl<'a> SignTransaction for LocalTxSigner<'a> {
    fn sign_transaction_with_inputs(
        &self,
        unsigned_tx: &UnsignedTransaction,
        inputs: TxIoVec<ErgoBox>,
        data_boxes: Option<TxIoVec<ErgoBox>>,
    ) -> Result<ergo_lib::chain::transaction::Transaction, NodeError> {
        let tx = self
            .wallet
            .sign_transaction(
                TransactionContext::new(
                    unsigned_tx.clone(),
                    inputs.as_vec().clone(),
                    data_boxes.map(|bs| bs.as_vec().clone()).unwrap_or_default(),
                )
                .unwrap(),
                self.ctx,
                None,
            )
            .unwrap();
        Ok(tx)
    }
}

pub fn init_log_tests() {
    // set log level via RUST_LOG=info env var
    let _ = env_logger::builder().is_test(true).try_init().unwrap();
}

pub fn make_oracle_contract_parameters() -> OracleContractParameters {
    // via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAob3JhY2xlIGJveCkKICAvLyAgIFI0IHB1YmxpYyBrZXkgKEdyb3VwRWxlbWVudCkgCiAgLy8gICBSNSBlcG9jaCBjb3VudGVyIG9mIGN1cnJlbnQgZXBvY2ggKEludCkKICAvLyAgIFI2IGRhdGEgcG9pbnQgKExvbmcpIG9yIGVtcHR5CgogIC8vICAgdG9rZW5zKDApIG9yYWNsZSB0b2tlbiAob25lKQogIC8vICAgdG9rZW5zKDEpIHJld2FyZCB0b2tlbnMgY29sbGVjdGVkIChvbmUgb3IgbW9yZSkgCiAgLy8gICAKICAvLyAgIFdoZW4gcHVibGlzaGluZyBhIGRhdGFwb2ludCwgdGhlcmUgbXVzdCBiZSBhdCBsZWFzdCBvbmUgcmV3YXJkIHRva2VuIGF0IGluZGV4IDEgCiAgLy8gIAogIC8vICAgV2Ugd2lsbCBjb25uZWN0IHRoaXMgYm94IHRvIHBvb2wgTkZUIGluIGlucHV0ICMwIChhbmQgbm90IHRoZSByZWZyZXNoIE5GVCBpbiBpbnB1dCAjMSkKICAvLyAgIFRoaXMgd2F5LCB3ZSBjYW4gY29udGludWUgdG8gdXNlIHRoZSBzYW1lIGJveCBhZnRlciB1cGRhdGluZyBwb29sCiAgLy8gICBUaGlzICpjb3VsZCogYWxsb3cgdGhlIG9yYWNsZSBib3ggdG8gYmUgc3BlbnQgZHVyaW5nIGFuIHVwZGF0ZQogIC8vICAgSG93ZXZlciwgdGhpcyBpcyBub3QgYW4gaXNzdWUgYmVjYXVzZSB0aGUgdXBkYXRlIGNvbnRyYWN0IGVuc3VyZXMgdGhhdCB0b2tlbnMgYW5kIHJlZ2lzdGVycyAoZXhjZXB0IHNjcmlwdCkgb2YgdGhlIHBvb2wgYm94IGFyZSBwcmVzZXJ2ZWQKCiAgLy8gICBQcml2YXRlIGtleSBob2xkZXIgY2FuIGRvIGZvbGxvd2luZyB0aGluZ3M6CiAgLy8gICAgIDEuIENoYW5nZSBncm91cCBlbGVtZW50IChwdWJsaWMga2V5KSBzdG9yZWQgaW4gUjQKICAvLyAgICAgMi4gU3RvcmUgYW55IHZhbHVlIG9mIHR5cGUgaW4gb3IgZGVsZXRlIGFueSB2YWx1ZSBmcm9tIFI0IHRvIFI5IAogIC8vICAgICAzLiBTdG9yZSBhbnkgdG9rZW4gb3Igbm9uZSBhdCAybmQgaW5kZXggCgogIC8vICAgSW4gb3JkZXIgdG8gY29ubmVjdCB0aGlzIG9yYWNsZSBib3ggdG8gYSBkaWZmZXJlbnQgcmVmcmVzaE5GVCBhZnRlciBhbiB1cGRhdGUsIAogIC8vICAgdGhlIG9yYWNsZSBzaG91bGQga2VlcCBhdCBsZWFzdCBvbmUgbmV3IHJld2FyZCB0b2tlbiBhdCBpbmRleCAxIHdoZW4gcHVibGlzaGluZyBkYXRhLXBvaW50CiAgCiAgdmFsIHBvb2xORlQgPSBmcm9tQmFzZTY0KCJSeXRMWWxCbFUyaFdiVmx4TTNRMmR6bDZKRU1tUmlsS1FFMWpVV1pVYWxjPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKICAKICB2YWwgb3RoZXJUb2tlbklkID0gSU5QVVRTKDApLnRva2VucygwKS5fMQogIAogIHZhbCBtaW5TdG9yYWdlUmVudCA9IDEwMDAwMDAwTAogIHZhbCBzZWxmUHViS2V5ID0gU0VMRi5SNFtHcm91cEVsZW1lbnRdLmdldAogIHZhbCBvdXRJbmRleCA9IGdldFZhcltJbnRdKDApLmdldAogIHZhbCBvdXRwdXQgPSBPVVRQVVRTKG91dEluZGV4KQogIAogIHZhbCBpc1NpbXBsZUNvcHkgPSBvdXRwdXQudG9rZW5zKDApID09IFNFTEYudG9rZW5zKDApICAgICAgICAgICAgICAgICYmIC8vIG9yYWNsZSB0b2tlbiBpcyBwcmVzZXJ2ZWQKICAgICAgICAgICAgICAgICAgICAgb3V0cHV0LnByb3Bvc2l0aW9uQnl0ZXMgPT0gU0VMRi5wcm9wb3NpdGlvbkJ5dGVzICAmJiAvLyBzY3JpcHQgcHJlc2VydmVkCiAgICAgICAgICAgICAgICAgICAgIG91dHB1dC5SNFtHcm91cEVsZW1lbnRdLmlzRGVmaW5lZCAgICAgICAgICAgICAgICAgJiYgLy8gb3V0cHV0IG11c3QgaGF2ZSBhIHB1YmxpYyBrZXkgKG5vdCBuZWNlc3NhcmlseSB0aGUgc2FtZSkKICAgICAgICAgICAgICAgICAgICAgb3V0cHV0LnZhbHVlID49IG1pblN0b3JhZ2VSZW50ICAgICAgICAgICAgICAgICAgICAgICAvLyBlbnN1cmUgc3VmZmljaWVudCBFcmdzIHRvIGVuc3VyZSBubyBnYXJiYWdlIGNvbGxlY3Rpb24KICAgICAgICAgICAgICAgICAgICAgCiAgdmFsIGNvbGxlY3Rpb24gPSBvdGhlclRva2VuSWQgPT0gcG9vbE5GVCAgICAgICAgICAgICAgICAgICAgJiYgLy8gZmlyc3QgaW5wdXQgbXVzdCBiZSBwb29sIGJveAogICAgICAgICAgICAgICAgICAgb3V0cHV0LnRva2VucygxKS5fMSA9PSBTRUxGLnRva2VucygxKS5fMSAgICYmIC8vIHJld2FyZCB0b2tlbklkIGlzIHByZXNlcnZlZCAob3JhY2xlIHNob3VsZCBlbnN1cmUgdGhpcyBjb250YWlucyBhIHJld2FyZCB0b2tlbikKICAgICAgICAgICAgICAgICAgIG91dHB1dC50b2tlbnMoMSkuXzIgPiBTRUxGLnRva2VucygxKS5fMiAgICAmJiAvLyBhdCBsZWFzdCBvbmUgcmV3YXJkIHRva2VuIG11c3QgYmUgYWRkZWQgCiAgICAgICAgICAgICAgICAgICBvdXRwdXQuUjRbR3JvdXBFbGVtZW50XS5nZXQgPT0gc2VsZlB1YktleSAgJiYgLy8gZm9yIGNvbGxlY3Rpb24gcHJlc2VydmUgcHVibGljIGtleQogICAgICAgICAgICAgICAgICAgb3V0cHV0LnZhbHVlID49IFNFTEYudmFsdWUgICAgICAgICAgICAgICAgICYmIC8vIG5hbm9FcmdzIHZhbHVlIHByZXNlcnZlZAogICAgICAgICAgICAgICAgICAgISAob3V0cHV0LlI1W0FueV0uaXNEZWZpbmVkKSAgICAgICAgICAgICAgICAgIC8vIG5vIG1vcmUgcmVnaXN0ZXJzOyBwcmV2ZW50cyBib3ggZnJvbSBiZWluZyByZXVzZWQgYXMgYSB2YWxpZCBkYXRhLXBvaW50CgogIHZhbCBvd25lciA9IHByb3ZlRGxvZyhzZWxmUHViS2V5KSAgCgogIC8vIG93bmVyIGNhbiBjaG9vc2UgdG8gdHJhbnNmZXIgdG8gYW5vdGhlciBwdWJsaWMga2V5IGJ5IHNldHRpbmcgZGlmZmVyZW50IHZhbHVlIGluIFI0CiAgaXNTaW1wbGVDb3B5ICYmIChvd25lciB8fCBjb2xsZWN0aW9uKSAKfQ==
    let address = AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str("2vTHJzWVd7ryXrP3fH9KfEFGzS8XFdVY99xXuxMPt664HurrUn3e8y3W1wTQDVZsDi9TDeZdun2XEr3pcipGmKdmciSADmKn32Cs8YuPLNp4zaBZNo6m6NG8tz3zznb56nRCrz5VDDjxYTsQ92DqhtQmG3m7H6zbtNHLzJjf7x9ZSD3vNWRL6e7usRjfm1diob8bdizsbJM7wNDzLZYhshHScEkWse9MQKgMDN4pYb1vQLR1PmvUnpsRAjRYwNBs3ZjJoqdSpN6jbjfSJsrgEhBANbnCZxP3dKBr").unwrap();
    OracleContractParameters {
        p2s: NetworkAddress::new(NetworkPrefix::Mainnet, &address),
        pool_nft_index: 5,
        pool_nft_token_id: force_any_val::<TokenId>(),
    }
}

pub fn make_pool_contract_parameters() -> PoolContractParameters {
    // via
    // https://wallet.plutomonkey.com/p2s/?source=ewogIC8vIFRoaXMgYm94IChwb29sIGJveCkKICAvLyAgIGVwb2NoIHN0YXJ0IGhlaWdodCBpcyBzdG9yZWQgaW4gY3JlYXRpb24gSGVpZ2h0IChSMykKICAvLyAgIFI0IEN1cnJlbnQgZGF0YSBwb2ludCAoTG9uZykKICAvLyAgIFI1IEN1cnJlbnQgZXBvY2ggY291bnRlciAoSW50KQogIC8vCiAgLy8gICB0b2tlbnMoMCkgcG9vbCB0b2tlbiAoTkZUKQogIC8vICAgdG9rZW5zKDEpIHJld2FyZCB0b2tlbnMKICAvLyAgIFdoZW4gaW5pdGlhbGl6aW5nIHRoZSBib3gsIHRoZXJlIG11c3QgYmUgb25lIHJld2FyZCB0b2tlbi4gV2hlbiBjbGFpbWluZyByZXdhcmQsIG9uZSB0b2tlbiBtdXN0IGJlIGxlZnQgdW5jbGFpbWVkCiAgCiAgdmFsIG90aGVyVG9rZW5JZCA9IElOUFVUUygxKS50b2tlbnMoMCkuXzEKICB2YWwgcmVmcmVzaE5GVCA9IGZyb21CYXNlNjQoIlZHcFhibHB5TkhVM2VDRkJKVVFxUnkxTFlVNWtVbWRWYTFod01uTTFkamc9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIHVwZGF0ZU5GVCA9IGZyb21CYXNlNjQoIllsRmxWR2hYYlZweE5IUTNkeUY2SlVNcVJpMUtRRTVqVW1aVmFsaHVNbkk9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCgogIHNpZ21hUHJvcChvdGhlclRva2VuSWQgPT0gcmVmcmVzaE5GVCB8fCBvdGhlclRva2VuSWQgPT0gdXBkYXRlTkZUKQp9
    let address = AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str("PViBL5acX6PoP6BQPsYtyNzW9aPXwxpRaUkXo4nE7RkxcBbZXJECUEBQm4g3MQCb2QsQALqPkrDN9TvsKuQkChF8sZSfnH5fifgKAkXhW8ifAcAE1qA67n9mabB3Mb2R8xT2v3SN49eN8mQ8HN95").unwrap();
    PoolContractParameters {
        p2s: NetworkAddress::new(NetworkPrefix::Mainnet, &address),
        refresh_nft_index: 2,
        refresh_nft_token_id: force_any_val::<TokenId>(),
        update_nft_index: 3,
        update_nft_token_id: force_any_val::<TokenId>(),
    }
}

pub fn make_refresh_contract_parameters() -> RefreshContractParameters {
    // v2.0a from https://github.com/scalahub/OraclePool/blob/v2/src/main/scala/oraclepool/v2a/Contracts.scala
    // compiled via
    // https://wallet.plutomonkey.com/p2s/?source=eyAvLyBUaGlzIGJveCAocmVmcmVzaCBib3gpCiAgLy8gICB0b2tlbnMoMCkgcmV3YXJkIHRva2VucyB0byBiZSBlbWl0dGVkIChzZXZlcmFsKSAKICAvLyAgIAogIC8vICAgV2hlbiBpbml0aWFsaXppbmcgdGhlIGJveCwgdGhlcmUgbXVzdCBiZSBvbmUgcmV3YXJkIHRva2VuLiBXaGVuIGNsYWltaW5nIHJld2FyZCwgb25lIHRva2VuIG11c3QgYmUgbGVmdCB1bmNsYWltZWQgICAKICAKICB2YWwgb3JhY2xlVG9rZW5JZCA9IGZyb21CYXNlNjQoIktrY3RTbUZPWkZKblZXdFljREp6TlhZNGVTOUNQMFVvU0N0TllsQmxVMmc9IikgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIHBvb2xORlQgPSBmcm9tQmFzZTY0KCJSeXRMWWxCbFUyaFdiVmx4TTNRMmR6bDZKRU1tUmlsS1FFMWpVV1pVYWxjPSIpIC8vIFRPRE8gcmVwbGFjZSB3aXRoIGFjdHVhbCAKICB2YWwgZXBvY2hMZW5ndGggPSAzMCAvLyBUT0RPIHJlcGxhY2Ugd2l0aCBhY3R1YWwKICB2YWwgbWluRGF0YVBvaW50cyA9IDQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIGJ1ZmZlciA9IDQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCiAgdmFsIG1heERldmlhdGlvblBlcmNlbnQgPSA1IC8vIHBlcmNlbnQgLy8gVE9ETyByZXBsYWNlIHdpdGggYWN0dWFsCgogIHZhbCBtaW5TdGFydEhlaWdodCA9IEhFSUdIVCAtIGVwb2NoTGVuZ3RoCiAgdmFsIHNwZW5kZXJJbmRleCA9IGdldFZhcltJbnRdKDApLmdldCAvLyB0aGUgaW5kZXggb2YgdGhlIGRhdGEtcG9pbnQgYm94IChOT1QgaW5wdXQhKSBiZWxvbmdpbmcgdG8gc3BlbmRlciAgICAKICAgIAogIHZhbCBwb29sSW4gPSBJTlBVVFMoMCkKICB2YWwgcG9vbE91dCA9IE9VVFBVVFMoMCkKICB2YWwgc2VsZk91dCA9IE9VVFBVVFMoMSkKCiAgZGVmIGlzVmFsaWREYXRhUG9pbnQoYjogQm94KSA9IGlmIChiLlI2W0xvbmddLmlzRGVmaW5lZCkgewogICAgYi5jcmVhdGlvbkluZm8uXzEgICAgPj0gbWluU3RhcnRIZWlnaHQgJiYgIC8vIGRhdGEgcG9pbnQgbXVzdCBub3QgYmUgdG9vIG9sZAogICAgYi50b2tlbnMoMCkuXzEgICAgICAgPT0gb3JhY2xlVG9rZW5JZCAgJiYgLy8gZmlyc3QgdG9rZW4gaWQgbXVzdCBiZSBvZiBvcmFjbGUgdG9rZW4KICAgIGIuUjVbSW50XS5nZXQgICAgICAgID09IHBvb2xJbi5SNVtJbnRdLmdldCAvLyBpdCBtdXN0IGNvcnJlc3BvbmQgdG8gdGhpcyBlcG9jaAogIH0gZWxzZSBmYWxzZSAKICAgICAgICAgIAogIHZhbCBkYXRhUG9pbnRzID0gSU5QVVRTLmZpbHRlcihpc1ZhbGlkRGF0YVBvaW50KSAgICAKICB2YWwgcHViS2V5ID0gZGF0YVBvaW50cyhzcGVuZGVySW5kZXgpLlI0W0dyb3VwRWxlbWVudF0uZ2V0CgogIHZhbCBlbm91Z2hEYXRhUG9pbnRzID0gZGF0YVBvaW50cy5zaXplID49IG1pbkRhdGFQb2ludHMgICAgCiAgdmFsIHJld2FyZEVtaXR0ZWQgPSBkYXRhUG9pbnRzLnNpemUgKiAyIC8vIG9uZSBleHRyYSB0b2tlbiBmb3IgZWFjaCBjb2xsZWN0ZWQgYm94IGFzIHJld2FyZCB0byBjb2xsZWN0b3IgICAKICB2YWwgZXBvY2hPdmVyID0gcG9vbEluLmNyZWF0aW9uSW5mby5fMSA8IG1pblN0YXJ0SGVpZ2h0CiAgICAgICAKICB2YWwgc3RhcnREYXRhID0gMUwgLy8gd2UgZG9uJ3QgYWxsb3cgMCBkYXRhIHBvaW50cwogIHZhbCBzdGFydFN1bSA9IDBMIAogIC8vIHdlIGV4cGVjdCBkYXRhLXBvaW50cyB0byBiZSBzb3J0ZWQgaW4gSU5DUkVBU0lORyBvcmRlcgogIAogIHZhbCBsYXN0U29ydGVkU3VtID0gZGF0YVBvaW50cy5mb2xkKChzdGFydERhdGEsICh0cnVlLCBzdGFydFN1bSkpLCB7CiAgICAgICAgKHQ6IChMb25nLCAoQm9vbGVhbiwgTG9uZykpLCBiOiBCb3gpID0+CiAgICAgICAgICAgdmFsIGN1cnJEYXRhID0gYi5SNltMb25nXS5nZXQKICAgICAgICAgICB2YWwgcHJldkRhdGEgPSB0Ll8xCiAgICAgICAgICAgdmFsIHdhc1NvcnRlZCA9IHQuXzIuXzEgCiAgICAgICAgICAgdmFsIG9sZFN1bSA9IHQuXzIuXzIKICAgICAgICAgICB2YWwgbmV3U3VtID0gb2xkU3VtICsgY3VyckRhdGEgIC8vIHdlIGRvbid0IGhhdmUgdG8gd29ycnkgYWJvdXQgb3ZlcmZsb3csIGFzIGl0IGNhdXNlcyBzY3JpcHQgdG8gZmFpbAoKICAgICAgICAgICB2YWwgaXNTb3J0ZWQgPSB3YXNTb3J0ZWQgJiYgcHJldkRhdGEgPD0gY3VyckRhdGEgCgogICAgICAgICAgIChjdXJyRGF0YSwgKGlzU29ydGVkLCBuZXdTdW0pKQogICAgfQogICkKIAogIHZhbCBsYXN0RGF0YSA9IGxhc3RTb3J0ZWRTdW0uXzEKICB2YWwgaXNTb3J0ZWQgPSBsYXN0U29ydGVkU3VtLl8yLl8xCiAgdmFsIHN1bSA9IGxhc3RTb3J0ZWRTdW0uXzIuXzIKICB2YWwgYXZlcmFnZSA9IHN1bSAvIGRhdGFQb2ludHMuc2l6ZSAKCiAgdmFsIG1heERlbHRhID0gbGFzdERhdGEgKiBtYXhEZXZpYXRpb25QZXJjZW50IC8gMTAwICAgICAgICAgIAogIHZhbCBmaXJzdERhdGEgPSBkYXRhUG9pbnRzKDApLlI2W0xvbmddLmdldAoKICBwcm92ZURsb2cocHViS2V5KSAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICYmCiAgZXBvY2hPdmVyICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAmJiAKICBlbm91Z2hEYXRhUG9pbnRzICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICYmICAgIAogIGlzU29ydGVkICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgJiYKICBsYXN0RGF0YSAtIGZpcnN0RGF0YSAgICAgPD0gbWF4RGVsdGEgICAgICAgICAgICAgICAgICAgICAgICAgICYmIAogIHBvb2xJbi50b2tlbnMoMCkuXzEgICAgICA9PSBwb29sTkZUICAgICAgICAgICAgICAgICAgICAgICAgICAgJiYKICBwb29sT3V0LnRva2VucygwKSAgICAgICAgPT0gcG9vbEluLnRva2VucygwKSAgICAgICAgICAgICAgICAgICYmIC8vIHByZXNlcnZlIHBvb2wgTkZUCiAgcG9vbE91dC50b2tlbnMoMSkuXzEgICAgID09IHBvb2xJbi50b2tlbnMoMSkuXzEgICAgICAgICAgICAgICAmJiAvLyByZXdhcmQgdG9rZW4gaWQgcHJlc2VydmVkCiAgcG9vbE91dC50b2tlbnMoMSkuXzIgICAgID49IHBvb2xJbi50b2tlbnMoMSkuXzIgLSByZXdhcmRFbWl0dGVkICYmIC8vIHJld2FyZCB0b2tlbiBhbW91bnQgY29ycmVjdGx5IHJlZHVjZWQKICBwb29sT3V0LnRva2Vucy5zaXplICAgICAgICA9PSBwb29sSW4udG9rZW5zLnNpemUgICAgICAgICAgICAgICYmIC8vIGNhbm5vdCBpbmplY3QgbW9yZSB0b2tlbnMgdG8gcG9vbCBib3gKICBwb29sT3V0LlI0W0xvbmddLmdldCAgICAgPT0gYXZlcmFnZSAgICAgICAgICAgICAgICAgICAgICAgICAgICYmIC8vIHJhdGUKICBwb29sT3V0LlI1W0ludF0uZ2V0ICAgICAgPT0gcG9vbEluLlI1W0ludF0uZ2V0ICsgMSAgICAgICAgICAgICYmIC8vIGNvdW50ZXIKICBwb29sT3V0LnByb3Bvc2l0aW9uQnl0ZXMgPT0gcG9vbEluLnByb3Bvc2l0aW9uQnl0ZXMgICAgICAgICAgICYmIC8vIHByZXNlcnZlIHBvb2wgc2NyaXB0CiAgcG9vbE91dC52YWx1ZSAgICAgICAgICAgID49IHBvb2xJbi52YWx1ZSAgICAgICAgICAgICAgICAgICAgICAmJgogIHBvb2xPdXQuY3JlYXRpb25JbmZvLl8xICA+PSBIRUlHSFQgLSBidWZmZXIgICAgICAgICAgICAgICAgICAgJiYgLy8gZW5zdXJlIHRoYXQgbmV3IGJveCBoYXMgY29ycmVjdCBzdGFydCBlcG9jaCBoZWlnaHQKICBzZWxmT3V0LnRva2VucyAgICAgICAgICAgPT0gU0VMRi50b2tlbnMgICAgICAgICAgICAgICAgICAgICAgICYmIC8vIHJlZnJlc2ggTkZUIHByZXNlcnZlZAogIHNlbGZPdXQucHJvcG9zaXRpb25CeXRlcyA9PSBTRUxGLnByb3Bvc2l0aW9uQnl0ZXMgICAgICAgICAgICAgJiYgLy8gc2NyaXB0IHByZXNlcnZlZAogIHNlbGZPdXQudmFsdWUgICAgICAgICAgICA+PSBTRUxGLnZhbHVlICAgICAgICAgICAgICAgICAgICAgICAKfQ==_
    let address = AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str("oq3jWGvabYxVYtceq1RGzFD4UdcdHcqY861G7H4mDiEnYQHya17A2w5r7u45moTpjAqfsNTm2XyhRNvYHiZhDTpmnfVa9XHSsbs5zjEw5UmgQfuP5d3NdFVy7oiAvLP1sjZN8qiHryzFoenLgtsxV8wLAeBaRChy73dd3rgyVfZipVL5LCXQyXMqp9oFFzPtTPkBw3ha7gJ4Bs5KjeUkVXJRVQ2Tdhg51Sdb6fEkHRtRuvCpynxYokQXP6SNif1M6mPcBR3B4zMLcFvmGxwNkZ3mRFzqHVzHV8Syu5AzueJEmMTrvWAXnhpYE7WcFbmDt3dqyXq7x9DNyKq1VwRwgFscLYDenAHqqHKd3jsJ6Grs8uFvvvJGKdqzdoJ3qCcCRXeDcZAKmExJMH4hJbsk8b1ct5YDBcNrq3LUr319XkS8miZDbHdHa88MSpCJQJmE51hmWVAV1yXrpyxqXqAXXPpSaGCP38BwCv8hYFK37DyA4mQd5r7vF9vNo5DEXwQ5wA2EivwRtNqpKUxXtKuZWTNC7Pu7NmvEHSuJPnaoCUujCiPtLM4dR64u8Gp7X3Ujo3o9zuMc6npemx3hf8rQS18QXgKJLwfeSqVYkicbVcGZRHsPsGxwrf1Wixp45E8d5e97MsKTCuqSskPKaHUdQYW1JZ8djcr4dxg1qQN81m7u2q8dwW6AK32mwRSS3nj27jkjML6n6GBpNZk9AtB2uMx3CHo6pZSaxgeCXuu3amrdeYmbuSqHUNZHU").unwrap();
    RefreshContractParameters {
        p2s: NetworkAddress::new(NetworkPrefix::Mainnet, &address),
        refresh_nft_token_id: force_any_val::<TokenId>(),
        pool_nft_index: 17,
        pool_nft_token_id: force_any_val::<TokenId>(),
        oracle_token_id_index: 3,
        oracle_token_id: force_any_val::<TokenId>(),
        min_data_points_index: 13,
        min_data_points: 4,
        buffer_index: 21,
        buffer_length: 4,
        max_deviation_percent_index: 15,
        max_deviation_percent: 5,
        epoch_length_index: 0,
        epoch_length: 30,
    }
}
