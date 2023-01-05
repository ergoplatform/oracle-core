use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;

pub mod bootstrap;
pub mod extract_reward_tokens;
pub mod import_pool_update;
pub mod prepare_update;
pub mod print_reward_tokens;
pub mod transfer_oracle_token;
pub mod update_pool;
pub mod vote_update_pool;

pub(crate) fn ergo_explorer_transaction_link(tx_id_str: String, prefix: NetworkPrefix) -> String {
    let prefix_str = match prefix {
        NetworkPrefix::Mainnet => "explorer",
        NetworkPrefix::Testnet => "testnet",
    };
    let tx_id_str = tx_id_str.replace('"', ""); // Node interface returns Tx Id as a JSON string "TxId"
    format!(
        "https://{}.ergoplatform.com/en/transactions/{}",
        prefix_str, tx_id_str
    )
}
