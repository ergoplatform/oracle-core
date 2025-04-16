use crate::oracle_config::ORACLE_SECRETS;
use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::{Transaction, TxId};
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::{Token, TokenId};
use ergo_lib::wallet::box_selector::{
    BoxSelection, BoxSelector, BoxSelectorError, ErgoBoxAssets, SimpleBoxSelector,
};
use ergo_lib::wallet::signing::TransactionContext;
use ergo_lib::wallet::{Wallet, WalletError};
use ergo_node_interface::scanning::NodeError;
use ergo_node_interface::{NodeInterface, P2PKAddressString};
use reqwest::Url;
use thiserror::Error;

pub trait NodeApiTrait {
    fn get_unspent_boxes_by_address_with_token_filter_option(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
        filter_boxes_token_ids: Vec<TokenId>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError>;

    fn get_unspent_boxes_by_address(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError>;

    fn get_unspent_boxes_by_token_id(
        &self,
        token_id: &TokenId,
    ) -> Result<Vec<ErgoBox>, NodeApiError>;

    fn get_state_context(&self) -> Result<ErgoStateContext, NodeApiError>;

    fn get_wallet(&self) -> Result<Wallet, NodeApiError>;

    fn sign_transaction(
        &self,
        transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<Transaction, NodeApiError>;

    fn submit_transaction(&self, tx: &Transaction) -> Result<TxId, NodeApiError>;

    fn sign_and_submit_transaction(
        &self,
        transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<TxId, NodeApiError>;
}

impl NodeApiTrait for NodeApi {
    fn get_unspent_boxes_by_address_with_token_filter_option(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
        filter_boxes_token_ids: Vec<TokenId>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        self.get_unspent_boxes_by_address_with_token_filter_option(
            address,
            target_balance,
            target_tokens,
            filter_boxes_token_ids,
        )
    }

    fn get_unspent_boxes_by_address(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        self.get_unspent_boxes_by_address(address, target_balance, target_tokens)
    }

    fn get_unspent_boxes_by_token_id(
        &self,
        token_id: &TokenId,
    ) -> Result<Vec<ErgoBox>, NodeApiError> {
        self.get_all_unspent_boxes_by_token_id(token_id)
    }

    fn get_state_context(&self) -> Result<ErgoStateContext, NodeApiError> {
        self.get_state_context()
    }

    fn get_wallet(&self) -> Result<Wallet, NodeApiError> {
        self.get_wallet()
    }

    fn sign_transaction(
        &self,
        transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<Transaction, NodeApiError> {
        self.sign_transaction(transaction_context)
    }

    fn submit_transaction(&self, tx: &Transaction) -> Result<TxId, NodeApiError> {
        self.submit_transaction(tx)
    }

    fn sign_and_submit_transaction(
        &self,
        transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<TxId, NodeApiError> {
        self.sign_and_submit_transaction(transaction_context)
    }
}

pub struct NodeApi {
    pub node: NodeInterface,
}

impl NodeApi {
    pub fn new(node_url: &Url) -> Self {
        let node = NodeInterface::from_url("", node_url.clone());
        Self { node }
    }

    /// Get unspent boxes by address with token filter option
    pub fn get_unspent_boxes_by_address_with_token_filter_option(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
        filter_boxes_token_ids: Vec<TokenId>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        let default_limit = 100;
        let box_selector = SimpleBoxSelector::new();
        let mut unspent_boxes: Vec<ErgoBox> = vec![];
        let mut offset = 0;
        let mut selection: Option<Result<BoxSelection<ErgoBox>, BoxSelectorError>> = None;
        loop {
            let boxes = self
                .node
                .unspent_boxes_by_address(address, offset, default_limit);
            if boxes.is_ok() {
                let boxes_clone = boxes.unwrap().clone();
                if boxes_clone.is_empty() {
                    break;
                }
                for box_ in boxes_clone.iter() {
                    let tokens = box_.tokens().clone();
                    if tokens.is_none() {
                        unspent_boxes.push(box_.clone());
                    } else {
                        let tokens = tokens.unwrap().to_vec();
                        if tokens
                            .iter()
                            .any(|token| filter_boxes_token_ids.contains(&token.token_id))
                        {
                            continue;
                        }
                        unspent_boxes.push(box_.clone());
                    }
                }
                let local_selection = box_selector.select(
                    unspent_boxes.clone(),
                    target_balance,
                    target_tokens.as_slice(),
                );
                selection = Some(local_selection.clone());
                if local_selection.is_ok() {
                    break;
                }
                offset += default_limit;
            } else {
                break;
            }
        }
        log::trace!("get_unspent_boxes_by_address_with_token_filter_option for address: {:#?} and found {:#?} boxes", address, unspent_boxes.len());
        Ok(selection.unwrap()?.boxes.to_vec())
    }

    /// Get unspent boxes by address
    pub fn get_unspent_boxes_by_address(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        let default_limit = 100;
        let box_selector = SimpleBoxSelector::new();
        let mut unspent_boxes: Vec<ErgoBox> = vec![];
        let mut offset = 0;
        let mut selection: Option<Result<BoxSelection<ErgoBox>, BoxSelectorError>> = None;
        loop {
            let boxes = self
                .node
                .unspent_boxes_by_address(address, offset, default_limit);
            if boxes.is_ok() {
                let mut boxes_clone = boxes.unwrap().clone();
                if boxes_clone.is_empty() {
                    break;
                }
                unspent_boxes.append(&mut boxes_clone);
                let local_selection = box_selector.select(
                    unspent_boxes.clone(),
                    target_balance,
                    target_tokens.as_slice(),
                );
                selection = Some(local_selection.clone());
                if local_selection.is_ok() {
                    break;
                }
                offset += default_limit;
            } else {
                break;
            }
        }
        log::trace!(
            "get_unspent_boxes_by_address for address: {:#?} and found {:#?} boxes",
            address,
            unspent_boxes.len()
        );
        Ok(selection.unwrap()?.boxes.to_vec())
    }

    /// Get unspent boxes by token id
    pub fn get_all_unspent_boxes_by_token_id(
        &self,
        token_id: &TokenId,
    ) -> Result<Vec<ErgoBox>, NodeApiError> {
        let default_limit = 100;
        let mut unspent_boxes: Vec<ErgoBox> = vec![];
        let mut offset = 0;
        loop {
            let boxes = self
                .node
                .unspent_boxes_by_token_id(token_id, offset, default_limit);
            if boxes.is_ok() {
                let mut boxes_clone = boxes.unwrap().clone();
                if boxes_clone.is_empty() {
                    break;
                }
                unspent_boxes.append(&mut boxes_clone);
                offset += default_limit;
            } else {
                break;
            }
        }
        log::trace!(
            "get_unspent_boxes_by_token_id for token: {:#?} and found {:#?} boxes",
            token_id,
            unspent_boxes.len()
        );
        Ok(unspent_boxes)
    }

    /// Get the current state context of the Ergo blockchain.
    pub fn get_state_context(&self) -> Result<ErgoStateContext, NodeApiError> {
        Ok(self.node.get_state_context()?)
    }

    /// Get the wallet instance from the oracle secrets.
    pub fn get_wallet(&self) -> Result<Wallet, NodeApiError> {
        let secret = ORACLE_SECRETS.secret_key.clone();
        Ok(Wallet::from_secrets(vec![secret]))
    }

    /// Sign an `UnsignedTransaction` and return the signed `Transaction`.
    pub fn sign_transaction(
        &self,
        transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<Transaction, NodeApiError> {
        log::trace!(
            "Signing transaction: {}",
            serde_json::to_string_pretty(&transaction_context.spending_tx).unwrap()
        );
        let wallet = self.get_wallet()?;
        let signed_tx =
            wallet.sign_transaction(transaction_context, &self.node.get_state_context()?, None);
        match signed_tx {
            Ok(tx) => {
                log::trace!(
                    "Signed transaction: {}",
                    serde_json::to_string_pretty(&tx).unwrap()
                );
                Ok(tx)
            }
            Err(wallet_err) => {
                log::error!("Sign Transaction Failed: {}", wallet_err.to_string());
                Err(NodeApiError::WalletError(wallet_err))
            }
        }
    }

    /// Submit a signed `Transaction` to the mempool.
    pub fn submit_transaction(&self, tx: &Transaction) -> Result<TxId, NodeApiError> {
        Ok(self.node.submit_transaction(tx)?)
    }

    /// Sign an `UnsignedTransaction` and submit the signed `Transaction` to the mempool.
    pub fn sign_and_submit_transaction(
        &self,
        transaction_context: TransactionContext<UnsignedTransaction>,
    ) -> Result<TxId, NodeApiError> {
        let tx = self.sign_transaction(transaction_context)?;
        self.submit_transaction(&tx)
    }

    /// Waits for the indexer to sync. This function will block until the indexer is fully synced.
    pub fn wait_for_indexer_sync(&self) -> Result<(), NodeApiError> {
        let indexer_status = self.node.indexer_status()?;
        if indexer_status.is_sync {
            log::debug!("Your indexer is already synced.");
            return Ok(());
        }
        Ok(loop {
            if indexer_status.is_sync {
                log::debug!("Your indexer synced successfully.");
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        })
    }
}

#[derive(Debug, Error)]
pub enum NodeApiError {
    #[error("Node error: {0}")]
    NodeInterfaceError(#[from] NodeError),
    #[error("Wallet error: {0}")]
    WalletError(#[from] WalletError),
    #[error("AddressEncoder error: {0}")]
    AddressEncoderError(#[from] AddressEncoderError),
    #[error("no change address is set in node")]
    NoChangeAddressSetInNode,
}
