use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::wallet::box_selector::ErgoBoxAssets;

pub fn get_token_count(b: ErgoBox, token_id: TokenId) -> u64 {
    let mut count = 0;
    if let Some(tokens) = b.tokens() {
        for token in tokens {
            if token.token_id == token_id {
                count += token.amount.as_u64();
            }
        }
    }
    count
}
