use std::convert::TryInto;

use derive_more::From;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
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
use crate::box_kind::OracleBox;
use crate::oracle_state::DatapointStage;
use crate::oracle_state::LiveEpochStage;
use crate::oracle_state::StageError;
use crate::wallet::WalletDataSource;

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
    #[error("unexpected error: {0}")]
    Unexpected(String),
}

pub fn build_action<A: LiveEpochStage, B: DatapointStage, C: WalletDataSource>(
    cmd: PoolCommand,
    live_epoch_stage_src: A,
    datapoint_stage_src: B,
    wallet: C,
    height: u32,
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
    height: u32,
    change_address: Address,
) -> Result<RefreshAction, PoolCommandError> {
    let tx_fee = BoxValue::SAFE_USER_MIN;

    let in_pool_box = live_epoch_stage_src.get_pool_box()?;
    let in_refresh_box = live_epoch_stage_src.get_refresh_box()?;
    let mut in_oracle_boxes = datapoint_stage_src.get_oracle_datapoint_boxes()?;
    in_oracle_boxes.sort_by_key(|b| b.rate());
    let valid_in_oracle_boxes = filter_oracle_boxes(in_oracle_boxes);
    let rate = calc_pool_rate(valid_in_oracle_boxes.clone());
    let reward_decrement = valid_in_oracle_boxes.len() as u32 * 2;
    let out_pool_box = build_out_pool_box(in_pool_box.clone(), height, rate)?;
    let out_refresh_box = build_out_refresh_box(in_refresh_box.clone(), height, reward_decrement)?;
    let mut out_oracle_boxes = build_out_oracle_boxes(&valid_in_oracle_boxes)?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;

    let mut input_boxes = vec![in_pool_box, in_refresh_box];
    let mut valid_in_oracle_raw_boxes = valid_in_oracle_boxes
        .into_iter()
        .map(|ob| ob.get_box())
        .collect();
    input_boxes.append(&mut valid_in_oracle_raw_boxes);
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

fn filter_oracle_boxes(oracle_boxes: Vec<&dyn OracleBox>) -> Vec<&dyn OracleBox> {
    todo!()
}

fn calc_pool_rate(oracle_boxes: Vec<&dyn OracleBox>) -> u64 {
    todo!()
}

fn build_out_pool_box(
    in_pool_box: ErgoBox,
    creation_height: u32,
    rate: u64,
) -> Result<ErgoBoxCandidate, PoolCommandError> {
    todo!()
}

fn build_out_refresh_box(
    in_refresh_box: ErgoBox,
    creation_height: u32,
    reward_decrement: u32,
) -> Result<ErgoBoxCandidate, PoolCommandError> {
    todo!()
}

fn build_out_oracle_boxes(
    valid_oracle_boxes: &Vec<&dyn OracleBox>,
) -> Result<Vec<ErgoBoxCandidate>, PoolCommandError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::chain::transaction::TxIoVec;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenId;
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::sigma_protocol::dlog_group::EcPoint;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use sigma_test_util::force_any_val;
    use sigma_test_util::force_any_val_with;

    use crate::box_kind::OracleBoxWrapper;

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
        datapoints: Vec<OracleBoxWrapper>,
    }

    impl DatapointStage for DatapointStageMock {
        fn get_oracle_datapoint_boxes(
            &self,
        ) -> std::result::Result<Vec<&dyn OracleBox>, StageError> {
            let wrapped_boxes = self
                .datapoints
                .iter()
                .map(|b| b as &dyn OracleBox)
                .collect();
            Ok(wrapped_boxes)
        }
    }

    #[derive(Clone)]
    struct WalletDataMock {
        unspent_boxes: Vec<ErgoBox>,
    }

    impl WalletDataSource for WalletDataMock {
        fn get_unspent_wallet_boxes(&self) -> Result<Vec<ErgoBox>, NodeError> {
            Ok(self.unspent_boxes.clone())
        }
    }

    fn refresh_contract() -> ErgoTree {
        todo!()
    }

    fn pool_contract() -> ErgoTree {
        todo!()
    }

    fn make_refresh_box(
        refresh_nft: &TokenId,
        reward_token: Token,
        value: BoxValue,
        creation_height: u32,
    ) -> ErgoBox {
        let tokens = vec![
            Token::from((refresh_nft.clone(), 1u64.try_into().unwrap())),
            reward_token,
        ]
        .try_into()
        .unwrap();
        ErgoBox::new(
            value,
            refresh_contract(),
            Some(tokens),
            NonMandatoryRegisters::empty(),
            creation_height,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    fn make_pool_box(
        datapoint: i64,
        epoch_counter: i32,
        refresh_nft: TokenId,
        value: BoxValue,
        creation_height: u32,
    ) -> ErgoBox {
        let tokens = [Token::from((refresh_nft.clone(), 1u64.try_into().unwrap()))].into();
        ErgoBox::new(
            value,
            pool_contract(),
            Some(tokens),
            NonMandatoryRegisters::new(
                vec![
                    (NonMandatoryRegisterId::R4, Constant::from(datapoint)),
                    (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
                ]
                .into_iter()
                .collect(),
            )
            .unwrap(),
            creation_height,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    fn make_datapoint_box(
        pub_key: EcPoint,
        datapoint: i64,
        epoch_counter: i32,
        oracle_token_id: TokenId,
        reward_token: Token,
        value: BoxValue,
        creation_height: u32,
    ) -> ErgoBox {
        let tokens = vec![
            Token::from((oracle_token_id.clone(), 1u64.try_into().unwrap())),
            reward_token,
        ]
        .try_into()
        .unwrap();
        ErgoBox::new(
            value,
            pool_contract(),
            Some(tokens),
            NonMandatoryRegisters::new(
                vec![
                    (NonMandatoryRegisterId::R4, Constant::from(datapoint)),
                    (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
                ]
                .into_iter()
                .collect(),
            )
            .unwrap(),
            creation_height,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    fn find_input_boxes(
        tx: UnsignedTransaction,
        available_boxes: Vec<ErgoBox>,
    ) -> TxIoVec<ErgoBox> {
        todo!()
    }

    #[test]
    fn test_refresh_pool() {
        let height = 100u32;
        let oracle_token_id =
            TokenId::from_base64("YlFlVGhXbVpxNHQ3dyF6JUMqRi1KQE5jUmZValhuMnI=").unwrap();
        let reward_token_id =
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap();
        let refresh_nft =
            TokenId::from_base64("VGpXblpyNHU3eCFBJUQqRy1LYU5kUmdVa1hwMnM1djg=").unwrap();
        let in_refresh_box = make_refresh_box(
            &refresh_nft,
            Token::from((reward_token_id.clone(), 100u64.try_into().unwrap())).clone(),
            BoxValue::SAFE_USER_MIN,
            height - 10,
        );
        let in_pool_box = make_pool_box(1, 1, refresh_nft, BoxValue::SAFE_USER_MIN, height - 10);
        let oracle_pub_key = force_any_val::<EcPoint>();
        let in_oracle_box = make_datapoint_box(
            oracle_pub_key,
            1,
            1,
            oracle_token_id,
            Token::from((reward_token_id, 5u64.try_into().unwrap())),
            BoxValue::SAFE_USER_MIN,
            height - 9, // right after the pool+oracle boxes block
        );
        let live_epoch_stage_mock = LiveEpochStageMock {
            refresh_box: in_refresh_box,
            pool_box: in_pool_box.clone(),
        };
        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();
        let datapoint_stage_mock = DatapointStageMock {
            datapoints: vec![OracleBoxWrapper::new(in_oracle_box.clone()).unwrap()],
        };
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![force_any_val_with::<ErgoBox>(
                (BoxValue::MIN_RAW * 5000..BoxValue::MIN_RAW * 10000).into(),
            )],
        };
        let action = build_refresh_action(
            live_epoch_stage_mock.clone(),
            datapoint_stage_mock.clone(),
            wallet_mock.clone(),
            100,
            change_address,
        )
        .unwrap();

        let ctx = force_any_val::<ErgoStateContext>();
        let wallet = Wallet::from_mnemonic("", "").unwrap();

        let mut possible_input_boxes = vec![
            live_epoch_stage_mock.get_pool_box().unwrap(),
            live_epoch_stage_mock.get_refresh_box().unwrap(),
        ];
        possible_input_boxes.append(&mut vec![in_oracle_box]);
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes),
            None,
        )
        .unwrap();
        assert!(wallet.sign_transaction(tx_context, &ctx, None).is_ok());
    }
}
