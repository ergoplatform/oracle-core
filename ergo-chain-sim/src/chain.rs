use ergo_lib::chain::transaction::Transaction;
use ergo_lib::ergotree_ir::chain::ergo_box::BoxId;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;

use crate::Block;

/// Ergo blockchain(UTXO) simulation
pub struct ChainSim {
    blocks: Vec<Block>,
    all_boxes: Vec<ErgoBox>,
    unspent_boxes: Vec<ErgoBox>,

    /// Current height
    pub height: u32,
}

impl ChainSim {
    fn get_unspent_box(&self, box_id: &BoxId) -> Option<ErgoBox> {
        for b in &self.unspent_boxes {
            if b.box_id() == *box_id {
                return Some(b.clone());
            }
        }
        None
    }

    fn get_box(&self, box_id: &BoxId) -> Option<ErgoBox> {
        for b in &self.all_boxes {
            if b.box_id() == *box_id {
                return Some(b.clone());
            }
        }
        None
    }

    fn update_utxo(&mut self, tx: Transaction) {
        let boxes_to_spend = tx
            .inputs
            .mapped(|i| self.get_unspent_box(&i.box_id).unwrap());
        let _data_input_boxes = tx
            .data_inputs
            .map(|data_inputs| data_inputs.mapped(|i| self.get_box(&i.box_id)));
        // TODO: verify tx signatures
        // TODO: verify tx using all the checks from https://github.com/ergoplatform/ergo/blob/1935c95560a30b19cdb52c1a291e8a389ba63c97/src/main/scala/org/ergoplatform/modifiers/mempool/ErgoTransaction.scala#L80-L384
        //
        self.unspent_boxes = self
            .unspent_boxes
            .clone()
            .into_iter()
            .filter(|b| !boxes_to_spend.as_vec().contains(b))
            .collect();
        self.all_boxes.append(tx.outputs.to_vec().as_mut());
    }

    /// Add a new block to the chain (head/latest)
    pub fn add_block(&mut self, block: Block) {
        block.txs.iter().for_each(|tx| {
            self.update_utxo(tx.clone());
        });
        self.blocks.push(block);
        self.height += 1;
    }
}
