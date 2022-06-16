use ergo_lib::chain::transaction::Transaction;

/// Block with transactions
pub struct Block {
    pub(crate) txs: Vec<Transaction>,
}

impl Block {
    /// Create a new block with the given transactions
    pub fn new(txs: Vec<Transaction>) -> Block {
        Block { txs }
    }
}
