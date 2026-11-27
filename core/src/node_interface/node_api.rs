use crate::oracle_config::ORACLE_CONFIG;
use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::{Transaction, TxId};
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
        let boxes = select_unspent_boxes_paginated(
            |offset, limit| self.node.unspent_boxes_by_address(address, offset, limit),
            target_balance,
            target_tokens.as_slice(),
            filter_boxes_token_ids.as_slice(),
        )?;
        log::trace!(
            "get_unspent_boxes_by_address_with_token_filter_option for address: {:#?} and selected {:#?} boxes",
            address,
            boxes.len()
        );
        Ok(boxes)
    }

    /// Get unspent boxes by address
    pub fn get_unspent_boxes_by_address(
        &self,
        address: &P2PKAddressString,
        target_balance: BoxValue,
        target_tokens: Vec<Token>,
    ) -> Result<Vec<ErgoBox>, BoxSelectorError> {
        let boxes = select_unspent_boxes_paginated(
            |offset, limit| self.node.unspent_boxes_by_address(address, offset, limit),
            target_balance,
            target_tokens.as_slice(),
            &[],
        )?;
        log::trace!(
            "get_unspent_boxes_by_address for address: {:#?} and selected {:#?} boxes",
            address,
            boxes.len()
        );
        Ok(boxes)
    }

    /// Get unspent boxes by token id
    pub fn get_all_unspent_boxes_by_token_id(
        &self,
        token_id: &TokenId,
    ) -> Result<Vec<ErgoBox>, NodeApiError> {
        let unspent_boxes = collect_paginated_boxes(|offset, limit| {
            self.node.unspent_boxes_by_token_id(token_id, offset, limit)
        })?;
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
        let secret = ORACLE_CONFIG.get_oracle_secret_key();
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
                log::error!("Sign Transaction Failed: {}", wallet_err);
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
}

fn select_unspent_boxes_paginated<F>(
    mut fetch_page: F,
    target_balance: BoxValue,
    target_tokens: &[Token],
    filter_boxes_token_ids: &[TokenId],
) -> Result<Vec<ErgoBox>, BoxSelectorError>
where
    F: FnMut(u64, u64) -> Result<Vec<ErgoBox>, NodeError>,
{
    let default_limit = 100;
    let box_selector = SimpleBoxSelector::new();
    let mut unspent_boxes: Vec<ErgoBox> = vec![];
    let mut offset = 0;
    let mut selection: Option<Result<BoxSelection<ErgoBox>, BoxSelectorError>> = None;
    loop {
        match fetch_page(offset, default_limit) {
            Ok(boxes) => {
                if boxes.is_empty() {
                    break;
                }
                for box_ in boxes {
                    let has_filtered_token = !filter_boxes_token_ids.is_empty()
                        && box_.tokens().as_ref().is_some_and(|tokens| {
                            tokens
                                .iter()
                                .any(|token| filter_boxes_token_ids.contains(&token.token_id))
                        });
                    if has_filtered_token {
                        continue;
                    }
                    unspent_boxes.push(box_);
                }
                let local_selection =
                    box_selector.select(unspent_boxes.clone(), target_balance, target_tokens);
                let is_ok = local_selection.is_ok();
                selection = Some(local_selection);
                if is_ok {
                    break;
                }
                offset += default_limit;
            }
            Err(err) => {
                log::error!(
                    "Failed to fetch unspent boxes at offset {offset} (limit {default_limit}): {err}"
                );
                break;
            }
        }
    }
    let selection = selection
        .unwrap_or_else(|| box_selector.select(unspent_boxes, target_balance, target_tokens))?;
    Ok(selection.boxes.to_vec())
}

fn collect_paginated_boxes<F>(mut fetch_page: F) -> Result<Vec<ErgoBox>, NodeApiError>
where
    F: FnMut(u64, u64) -> Result<Vec<ErgoBox>, NodeError>,
{
    let default_limit = 100;
    let mut unspent_boxes: Vec<ErgoBox> = vec![];
    let mut offset = 0;
    loop {
        let mut boxes = fetch_page(offset, default_limit)?;
        if boxes.is_empty() {
            break;
        }
        unspent_boxes.append(&mut boxes);
        offset += default_limit;
    }
    Ok(unspent_boxes)
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use sigma_test_util::force_any_val;

    use super::*;
    use crate::oracle_config::BASE_FEE;
    use crate::pool_commands::test_utils::make_wallet_unspent_box;

    #[test]
    fn test_collect_paginated_boxes_propagates_rpc_errors() {
        let err =
            collect_paginated_boxes(|_, _| Err(NodeError::Other("rpc down".into()))).unwrap_err();
        assert!(matches!(err, NodeApiError::NodeInterfaceError(_)));

        let secret = force_any_val::<DlogProverInput>();
        let page0 = vec![make_wallet_unspent_box(
            secret.public_image(),
            *BASE_FEE,
            None,
        )];
        let err_later = collect_paginated_boxes(|offset, _| {
            if offset == 0 {
                Ok(page0.clone())
            } else {
                Err(NodeError::Other("page 2 failed".into()))
            }
        })
        .unwrap_err();
        assert!(matches!(err_later, NodeApiError::NodeInterfaceError(_)));
    }

    #[test]
    fn test_collect_paginated_boxes_combines_pages() {
        let secret = force_any_val::<DlogProverInput>();
        let box0 = make_wallet_unspent_box(secret.public_image(), *BASE_FEE, None);
        let box1 = make_wallet_unspent_box(secret.public_image(), *BASE_FEE, None);
        let all = collect_paginated_boxes(|offset, _| match offset {
            0 => Ok(vec![box0.clone()]),
            100 => Ok(vec![box1.clone()]),
            _ => Ok(vec![]),
        })
        .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].box_id(), box0.box_id());
        assert_eq!(all[1].box_id(), box1.box_id());
    }

    #[test]
    fn test_select_unspent_boxes_paginated_returns_error_when_empty_or_rpc_fails() {
        let empty_res = select_unspent_boxes_paginated(|_, _| Ok(vec![]), *BASE_FEE, &[], &[]);
        assert!(matches!(
            empty_res,
            Err(BoxSelectorError::NotEnoughCoins(_))
        ));

        let rpc_err_res = select_unspent_boxes_paginated(
            |_, _| Err(NodeError::Other("node unreachable".into())),
            *BASE_FEE,
            &[],
            &[],
        );
        assert!(matches!(
            rpc_err_res,
            Err(BoxSelectorError::NotEnoughCoins(_))
        ));
    }

    #[test]
    fn test_select_unspent_boxes_paginated_filters_tokens_and_paginates() {
        use ergo_lib::ergo_chain_types::Digest32;

        let secret = force_any_val::<DlogProverInput>();
        let excluded_token_id = TokenId::from(Digest32::from([1u8; 32]));
        let wanted_token_id = TokenId::from(Digest32::from([2u8; 32]));

        let excluded_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10).unwrap(),
            Some(
                vec![Token {
                    token_id: excluded_token_id,
                    amount: 1u64.try_into().unwrap(),
                }]
                .try_into()
                .unwrap(),
            ),
        );
        let wanted_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10).unwrap(),
            Some(
                vec![Token {
                    token_id: wanted_token_id,
                    amount: 1u64.try_into().unwrap(),
                }]
                .try_into()
                .unwrap(),
            ),
        );

        let target_tokens = vec![Token {
            token_id: wanted_token_id,
            amount: 1u64.try_into().unwrap(),
        }];
        let selected = select_unspent_boxes_paginated(
            |offset, _| match offset {
                0 => Ok(vec![excluded_box.clone()]),
                100 => Ok(vec![wanted_box.clone()]),
                _ => Ok(vec![]),
            },
            *BASE_FEE,
            &target_tokens,
            &[excluded_token_id],
        )
        .unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].box_id(), wanted_box.box_id());

        // When all available boxes are filtered out, return Err instead of panicking
        let filtered_out = select_unspent_boxes_paginated(
            |offset, _| {
                if offset == 0 {
                    Ok(vec![excluded_box.clone()])
                } else {
                    Ok(vec![])
                }
            },
            *BASE_FEE,
            &[],
            &[excluded_token_id],
        );
        assert!(matches!(
            filtered_out,
            Err(BoxSelectorError::NotEnoughCoins(_))
        ));
    }
}
