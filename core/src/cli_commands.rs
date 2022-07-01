use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;

pub mod bootstrap;
pub mod extract_reward_tokens;
pub mod print_reward_tokens;
pub mod transfer_oracle_token;
pub mod vote_update_pool;

pub(crate) fn ergo_explorer_transaction_link(tx_id_str: String, prefix: NetworkPrefix) -> String {
    let prefix_str = match prefix {
        NetworkPrefix::Mainnet => "explorer",
        NetworkPrefix::Testnet => "testnet",
    };
    format!(
        "https://{}.ergoplatform.com/en/transactions/{}",
        prefix_str, tx_id_str
    )
}
