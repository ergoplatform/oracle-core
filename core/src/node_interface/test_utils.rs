use crate::node_interface::node_api::{NodeApiError, NodeApiTrait};
use ergo_chain_sim::{Block, ChainSim};
use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::{Transaction, TxId};
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
use ergo_lib::wallet::box_selector::BoxSelectorError;
use ergo_lib::wallet::secret_key::SecretKey;
use ergo_lib::wallet::signing::TransactionContext;
use ergo_lib::wallet::Wallet;
use ergo_node_interface::node_interface::NodeError;
use ergo_node_interface::P2PKAddressString;
use std::cell::RefCell;

pub struct ChainSubmitTx<'a> {
    pub(crate) chain: RefCell<&'a mut ChainSim>,
}

#[derive(Default)]
pub(crate) struct SubmitTxMock {
    pub(crate) transactions: RefCell<Vec<Transaction>>,
}

pub struct MockNodeApi<'a> {
    pub unspent_boxes: Vec<ErgoBox>,
    pub secrets: Vec<SecretKey>,
    pub submitted_txs: &'a RefCell<Vec<Transaction>>,
    pub chain_submit_tx: Option<&'a mut ChainSubmitTx<'a>>,
    pub ctx: ErgoStateContext,
}

impl NodeApiTrait for MockNodeApi<'_> {
    fn get_unspent_boxes_by_address_with_token_filter_option(
        &self,
        _address: &P2PKAddressString,
        _target_balance: BoxValue,
        _target_tokens: Vec<Token>,
        _filter_boxes_token_ids: Vec<TokenId>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        Ok(self.unspent_boxes.clone())
    }

    fn get_unspent_boxes_by_address(
        &self,
        _address: &P2PKAddressString,
        _target_balance: BoxValue,
        _target_tokens: Vec<Token>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        Ok(self.unspent_boxes.clone())
    }

    fn get_unspent_boxes_by_token_id(
        &self,
        _token_id: &TokenId,
    ) -> Result<Vec<ErgoBox>, NodeApiError> {
        Ok(self.unspent_boxes.clone())
    }

    fn get_state_context(&self) -> Result<ErgoStateContext, NodeApiError> {
        Ok(self.ctx.clone())
    }

    fn get_wallet(&self) -> Result<Wallet, NodeApiError> {
        let wallet = Wallet::from_secrets(self.secrets.clone());
        Ok(wallet)
    }

    fn sign_transaction(
        &self,
        _transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<Transaction, NodeApiError> {
        self.get_wallet()?
            .sign_transaction(_transaction_context, &self.ctx.clone(), None)
            .map_err(|e| NodeApiError::NodeInterfaceError(NodeError::Other(e.to_string())))
    }

    fn submit_transaction(&self, _tx: &Transaction) -> Result<TxId, NodeApiError> {
        self.submitted_txs.borrow_mut().push(_tx.clone());
        if let Some(ref chain_submit_tx) = &self.chain_submit_tx {
            chain_submit_tx
                .chain
                .borrow_mut()
                .add_block(Block::new(vec![_tx.clone()]));
        }
        Ok(_tx.id())
    }

    fn sign_and_submit_transaction(
        &self,
        _transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<TxId, NodeApiError> {
        self.sign_transaction(_transaction_context)
            .and_then(|tx| self.submit_transaction(&tx))
    }
}
