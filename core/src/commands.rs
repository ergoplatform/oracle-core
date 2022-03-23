use std::convert::TryInto;

use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::wallet::box_selector::BoxSelection;
use ergo_lib::wallet::box_selector::BoxSelector;
use ergo_lib::wallet::box_selector::BoxSelectorError;
use ergo_lib::wallet::box_selector::SimpleBoxSelector;
use ergo_lib::wallet::tx_builder::TxBuilder;
use ergo_lib::wallet::tx_builder::TxBuilderError;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use crate::actions::PoolAction;
use crate::actions::RefreshAction;
use crate::oracle_state::DatapointStage;
use crate::oracle_state::LiveEpochStage;
use crate::oracle_state::StageError;
use crate::wallet::WalletDataSource;
use crate::BlockHeight;

pub enum PoolCommand {
    Bootstrap,
    Refresh,
}

#[derive(Debug, From, Error)]
pub enum PoolCommandError {
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("tx builder error: {0}")]
    TxBuilderError(TxBuilderError),
    #[error("node error: {0}")]
    NodeError(NodeError),
    #[error("box selector error: {0}")]
    BoxSelectorError(BoxSelectorError),
}

pub fn build_action<A: LiveEpochStage, B: DatapointStage, C: WalletDataSource>(
    cmd: PoolCommand,
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
    wallet: C,
    height: BlockHeight,
    change_address: Address,
) -> Result<PoolAction, PoolCommandError> {
    match cmd {
        PoolCommand::Bootstrap => todo!(),
        PoolCommand::Refresh => build_refresh_action(
            live_epoch_stage_src,
            datapoint_stage_src,
            wallet,
            height,
            change_address,
        )
        .map(Into::into),
    }
}

pub fn build_refresh_action<A: LiveEpochStage, B: DatapointStage, C: WalletDataSource>(
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
    wallet: C,
    height: BlockHeight,
    change_address: Address,
) -> Result<RefreshAction, PoolCommandError> {
    let tx_fee = BoxValue::SAFE_USER_MIN;

    let in_pool_box = live_epoch_stage_src.get_pool_box()?;
    let in_refresh_box = live_epoch_stage_src.get_refresh_box()?;
    let mut in_oracle_boxes = datapoint_stage_src.get_oracle_datapoint_boxes()?;
    let out_pool_box = build_out_pool_box()?;
    let out_refresh_box = build_out_pool_box()?;
    let mut out_oracle_boxes = build_out_oracle_boxes()?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;

    let mut input_boxes = vec![in_pool_box, in_refresh_box];
    input_boxes.append(&mut in_oracle_boxes);
    input_boxes.append(selection.boxes.as_vec().clone().as_mut());
    let box_selection = BoxSelection {
        boxes: input_boxes.try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };

    let mut output_candidates = vec![out_pool_box, out_refresh_box];
    output_candidates.append(&mut out_oracle_boxes);

    let b = TxBuilder::new(
        box_selection,
        output_candidates,
        height as u32,
        tx_fee,
        change_address,
        BoxValue::MIN,
    );
    let tx = b.build()?;
    Ok(RefreshAction { tx })
}

fn build_out_pool_box() -> Result<ErgoBoxCandidate, PoolCommandError> {
    todo!()
}

fn build_out_refresh_box() -> Result<ErgoBoxCandidate, PoolCommandError> {
    todo!()
}

fn build_out_oracle_boxes() -> Result<Vec<ErgoBoxCandidate>, PoolCommandError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
    use ergo_lib::chain::transaction::TxIoVec;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenAmount;
    use ergo_lib::ergotree_ir::chain::token::TokenId;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use proptest::prelude::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    use crate::BlockHeight;

    use super::*;

    #[derive(Clone)]
    struct LiveEpochStageMock {
        refresh_box: ErgoBox,
        pool_box: ErgoBox,
    }

    impl LiveEpochStage for LiveEpochStageMock {
        fn get_refresh_box(&self) -> std::result::Result<ErgoBox, StageError> {
            Ok(self.refresh_box.clone())
        }

        fn get_pool_box(&self) -> std::result::Result<ErgoBox, StageError> {
            Ok(self.pool_box.clone())
        }
    }

    #[derive(Clone)]
    struct DatapointStageMock {
        datapoints: Vec<ErgoBox>,
    }

    impl DatapointStage for DatapointStageMock {
        fn get_oracle_datapoint_boxes(&self) -> std::result::Result<Vec<ErgoBox>, StageError> {
            Ok(self.datapoints.clone())
        }
    }

    #[derive(Clone)]
    struct WalletDataMock {}

    impl WalletDataSource for WalletDataMock {
        fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, NodeError> {
            todo!()
        }
    }

    fn make_refresh_box(refresh_nft: &TokenId, reward_token: Token, value: BoxValue) -> ErgoBox {
        todo!()
        // ErgoBox::new(
        //     value,
        //     ergo_tree,
        //     tokens,
        //     additional_registers,
        //     creation_height,
        //     transaction_id,
        //     index,
        // )
        // .unwrap()
    }

    fn make_pool_box(
        epoch_start_height: BlockHeight,
        datapoint: u64,
        epoch_counter: u32,
        refresh_nft: TokenId,
        value: BoxValue,
    ) -> ErgoBox {
        todo!()
    }

    fn make_datapoint_box() -> ErgoBox {
        todo!()
    }

    pub fn force_any_val<T: Arbitrary>() -> T {
        let mut runner = TestRunner::default();
        any::<T>().new_tree(&mut runner).unwrap().current()
    }

    #[test]
    fn test_refresh_pool() {
        let reward_token_id =
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap();
        let reward_token_amt: TokenAmount = 100u64.try_into().unwrap();
        let reward_token: Token = (reward_token_id, reward_token_amt).into();
        let refresh_nft =
            TokenId::from_base64("VGpXblpyNHU3eCFBJUQqRy1LYU5kUmdVa1hwMnM1djg=").unwrap();
        let refresh_box = make_refresh_box(&refresh_nft, reward_token, BoxValue::SAFE_USER_MIN);
        let pool_box = make_pool_box(1, 1, 1, refresh_nft, BoxValue::SAFE_USER_MIN);
        let datapoints = vec![make_datapoint_box()];
        let live_epoch_stage_mock = LiveEpochStageMock {
            refresh_box,
            pool_box: pool_box.clone(),
        };
        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();
        let datapoint_stage_mock = DatapointStageMock { datapoints };
        let wallet_mock = WalletDataMock {};
        let action = build_refresh_action(
            live_epoch_stage_mock.clone(),
            datapoint_stage_mock.clone(),
            wallet_mock.clone(),
            100,
            change_address,
        )
        .unwrap();
        // TODO: try to sign the tx

        let ctx = force_any_val::<ErgoStateContext>();
        let wallet = Wallet::from_mnemonic("", "").unwrap();

        let in_pool_box = live_epoch_stage_mock.get_pool_box().unwrap();
        let in_refresh_box = live_epoch_stage_mock.get_refresh_box().unwrap();
        let mut in_oracle_boxes = datapoint_stage_mock.get_oracle_datapoint_boxes().unwrap();
        let mut unspent_boxes = wallet_mock.get_unspent_wallet_boxes().unwrap();
        let mut input_boxes = vec![in_pool_box, in_refresh_box];
        input_boxes.append(&mut in_oracle_boxes);
        input_boxes.append(&mut unspent_boxes);

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, input_boxes),
            None,
        )
        .unwrap();
        assert!(wallet.sign_transaction(tx_context, &ctx, None).is_ok());
    }

    fn find_input_boxes(
        tx: UnsignedTransaction,
        available_boxes: Vec<ErgoBox>,
    ) -> TxIoVec<ErgoBox> {
        todo!()
    }
}
