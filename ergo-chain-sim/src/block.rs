use ergo_lib::chain::transaction::Transaction;

/// Block with transactions
pub struct Block {}

impl Block {
    /// Create a new block with the given transactions
    pub fn new(_txs: Vec<Transaction>) -> Block {
        Block {}
    }
}
