use ergo_lib::chain::transaction::Transaction;
use ergo_lib::chain::transaction::TxId;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::BoxId;
use ergo_lib::ergotree_ir::chain::ergo_box::BoxTokens;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;

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
        let boxes_to_spend = tx.inputs.mapped(|i| {
            self.get_unspent_box(&i.box_id)
                .ok_or(format!(
                    "cannot find {:?} in unspent boxes: {:?}",
                    &i.box_id, &self.unspent_boxes
                ))
                .unwrap()
        });
        let _data_input_boxes = tx
            .data_inputs
            .map(|data_inputs| data_inputs.mapped(|i| self.get_box(&i.box_id)));
        // TODO: verify tx signatures
        // TODO: verify tx using all the checks from https://github.com/ergoplatform/ergo/blob/1935c95560a30b19cdb52c1a291e8a389ba63c97/src/main/scala/org/ergoplatform/modifiers/mempool/ErgoTransaction.scala#L80-L384
        //
        dbg!("removing spent boxes from UTXO: {:?}", &boxes_to_spend);
        self.unspent_boxes = self
            .unspent_boxes
            .clone()
            .into_iter()
            .filter(|b| !boxes_to_spend.as_vec().contains(b))
            .collect();
        dbg!("adding tx outputs to UTXO: {:?}", &tx.outputs);
        self.unspent_boxes.append(tx.outputs.to_vec().as_mut());
        self.all_boxes.append(tx.outputs.to_vec().as_mut());
    }

    /// Create a new chain simulation
    pub fn new() -> ChainSim {
        ChainSim {
            blocks: Vec::new(),
            all_boxes: Vec::new(),
            unspent_boxes: Vec::new(),
            height: 0,
        }
    }

    /// Add a new block to the chain (head/latest)
    pub fn add_block(&mut self, block: Block) {
        block.txs.iter().for_each(|tx| {
            // dbg!(&tx);
            self.update_utxo(tx.clone());
        });
        self.blocks.push(block);
        self.height += 1;
    }

    /// Generates an unspent box guarded by a given ErgoTree holding a given assests
    pub fn generate_unspent_box(
        &mut self,
        ergo_tree: ErgoTree,
        value: BoxValue,
        tokens: Option<BoxTokens>,
    ) {
        let b = ErgoBox::new(
            value,
            ergo_tree,
            tokens,
            NonMandatoryRegisters::empty(),
            0,
            TxId::zero(),
            0,
        )
        .unwrap();
        self.unspent_boxes.push(b.clone());
        self.all_boxes.push(b);
    }

    /// Returns unspent boxes guarder by the given ErgoTree
    pub fn get_unspent_boxes(&self, ergo_tree: &ErgoTree) -> Vec<ErgoBox> {
        self.unspent_boxes
            .iter()
            .filter(|b| &b.ergo_tree == ergo_tree)
            .cloned()
            .collect()
    }
}

impl Default for ChainSim {
    fn default() -> Self {
        Self::new()
    }
}
