use crate::actions::RefreshAction;
use crate::box_kind::OracleBox;
use crate::box_kind::PoolBox;
use crate::box_kind::PoolBoxWrapper;
use crate::box_kind::RefreshBox;
use crate::box_kind::RefreshBoxWrapper;
use crate::contracts::refresh::RefreshContract;
use crate::oracle_state::DatapointBoxesSource;
use crate::oracle_state::PoolBoxSource;
use crate::oracle_state::RefreshBoxSource;
use crate::oracle_state::StageError;
use crate::wallet::WalletDataSource;

use derive_more::From;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_interpreter::sigma_protocol::prover::ContextExtension;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId::R4;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId::R5;
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::wallet::box_selector::BoxSelection;
use ergo_lib::wallet::box_selector::BoxSelector;
use ergo_lib::wallet::box_selector::BoxSelectorError;
use ergo_lib::wallet::box_selector::SimpleBoxSelector;
use ergo_lib::wallet::tx_builder::TxBuilder;
use ergo_lib::wallet::tx_builder::TxBuilderError;
use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

use std::convert::TryInto;

#[derive(Debug, From, Error)]
pub enum RefrechActionError {
    #[error("Failed collecting datapoints. The minimum consensus number could not be reached, meaning that an insufficient number of oracles posted datapoints within the deviation range: found {found}, expected {expected}")]
    FailedToReachConsensus { found: u32, expected: u32 },
    #[error("Not enough datapoints left during the removal of the outliers")]
    NotEnoughDatapoints,
    #[error("stage error: {0}")]
    StageError(StageError),
    #[error("node error: {0}")]
    NodeError(NodeError),
    #[error("box selector error: {0}")]
    BoxSelectorError(BoxSelectorError),
    #[error("tx builder error: {0}")]
    TxBuilderError(TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilderError(ErgoBoxCandidateBuilderError),
}

pub fn build_refresh_action(
    pool_box_source: &dyn PoolBoxSource,
    refresh_box_source: &dyn RefreshBoxSource,
    datapoint_stage_src: &dyn DatapointBoxesSource,
    wallet: &dyn WalletDataSource,
    height: u32,
    change_address: Address,
) -> Result<RefreshAction, RefrechActionError> {
    let tx_fee = BoxValue::SAFE_USER_MIN;

    let in_pool_box = pool_box_source.get_pool_box()?;
    let in_refresh_box = refresh_box_source.get_refresh_box()?;
    let in_oracle_boxes = datapoint_stage_src.get_oracle_datapoint_boxes()?;
    let refresh_contract = RefreshContract::new();
    let deviation_range = refresh_contract.max_deviation_percent();
    let valid_in_oracle_boxes = filtered_oracle_boxes(
        in_oracle_boxes
            .iter()
            .map(|b| b as &dyn OracleBox)
            .collect(),
        deviation_range,
    )?;
    if (valid_in_oracle_boxes.len() as u32) < RefreshContract::new().min_data_points() {
        return Err(RefrechActionError::FailedToReachConsensus {
            found: valid_in_oracle_boxes.len() as u32,
            expected: RefreshContract::new().min_data_points(),
        });
    }
    let rate = calc_pool_rate(valid_in_oracle_boxes.iter().map(|b| b.rate()).collect());
    let reward_decrement = valid_in_oracle_boxes.len() as u64 * 2;
    let out_pool_box = build_out_pool_box(&in_pool_box, height, rate)?;
    let out_refresh_box = build_out_refresh_box(&in_refresh_box, height, reward_decrement)?;
    let mut out_oracle_boxes = build_out_oracle_boxes(&valid_in_oracle_boxes, height)?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;

    let mut input_boxes = vec![in_pool_box.get_box(), in_refresh_box.get_box()];
    let mut valid_in_oracle_raw_boxes = valid_in_oracle_boxes
        .clone()
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

    let mut b = TxBuilder::new(
        box_selection,
        output_candidates,
        height as u32,
        tx_fee,
        change_address,
        BoxValue::MIN,
    );
    let in_refresh_box_ctx_ext = ContextExtension {
        values: vec![(0, 0i32.into())].into_iter().collect(),
    };
    b.set_context_extension(in_refresh_box.get_box().box_id(), in_refresh_box_ctx_ext);
    valid_in_oracle_boxes
        .iter()
        .enumerate()
        .for_each(|(idx, ob)| {
            let outindex = (idx as i32 + 2).into(); // first two output boxes are pool box and refresh box
            let ob_ctx_ext = ContextExtension {
                values: vec![(0, outindex)].into_iter().collect(),
            };
            b.set_context_extension(ob.get_box().box_id(), ob_ctx_ext);
        });
    let tx = b.build()?;
    Ok(RefreshAction { tx })
}

fn filtered_oracle_boxes(
    oracle_boxes: Vec<&dyn OracleBox>,
    deviation_range: u32,
) -> Result<Vec<&dyn OracleBox>, RefrechActionError> {
    // The oracle boxes must be arranged in increasing order of their R6 values (rate).
    let mut successful_boxes = oracle_boxes.clone();
    successful_boxes.sort_by_key(|b| b.rate());
    // The first oracle box's rate must be within deviation_range(5%) of that of the last, and must be > 0
    while !deviation_check(deviation_range, &successful_boxes) {
        // Removing largest deviation outlier
        remove_largest_local_deviation_datapoint(&mut successful_boxes)?;
    }
    Ok(successful_boxes)
}

fn deviation_check(deviation_range: u32, datapoint_boxes: &Vec<&dyn OracleBox>) -> bool {
    let num = datapoint_boxes.len();
    // expected sorted datapoint_boxes
    let max_datapoint = datapoint_boxes[0].rate();
    let min_datapoint = datapoint_boxes[num - 1].rate();
    let deviation_delta = max_datapoint * (deviation_range as u64) / 100;
    min_datapoint >= max_datapoint - deviation_delta
}

/// Finds whether the first or the last value in a list of sorted Datapoint boxes
/// deviates more compared to their adjacted datapoint, and then removes
/// said datapoint which deviates further.
fn remove_largest_local_deviation_datapoint(
    datapoint_boxes: &mut Vec<&dyn OracleBox>,
) -> Result<(), RefrechActionError> {
    let dp_len = datapoint_boxes.len();
    // Check if sufficient number of datapoint boxes to start removing
    if dp_len <= 2 {
        Err(RefrechActionError::NotEnoughDatapoints)
    } else {
        let datapoints: Vec<i64> = datapoint_boxes
            .iter()
            .map(|_| datapoint_boxes[0].rate() as i64)
            .collect();
        // Check deviation by subtracting largest value by 2nd largest
        let front_deviation = datapoints[0] - datapoints[1];
        // Check deviation by subtracting 2nd smallest value by smallest
        let back_deviation = datapoints[dp_len - 2] - datapoints[dp_len - 1];

        // Remove largest datapoint if front deviation is greater
        if front_deviation >= back_deviation {
            datapoint_boxes.drain(0..1);
        }
        // Remove smallest datapoint if back deviation is greater
        else {
            datapoint_boxes.pop();
        }
        Ok(())
    }
}

fn calc_pool_rate(oracle_boxes_rates: Vec<u64>) -> u64 {
    let datapoints_sum: u64 = oracle_boxes_rates.iter().sum();
    datapoints_sum / oracle_boxes_rates.len() as u64
}

fn build_out_pool_box(
    in_pool_box: &PoolBoxWrapper,
    creation_height: u32,
    rate: u64,
) -> Result<ErgoBoxCandidate, RefrechActionError> {
    let mut builder = ErgoBoxCandidateBuilder::new(
        in_pool_box.get_box().value,
        in_pool_box.get_box().ergo_tree.clone(),
        creation_height,
    );
    let new_epoch_counter: i32 = (in_pool_box.epoch_counter() + 1) as i32;
    builder.add_token(in_pool_box.pool_nft_token().clone());
    builder.set_register_value(R4, (rate as i64).into());
    builder.set_register_value(R5, new_epoch_counter.into());
    builder.build().map_err(Into::into)
}

fn build_out_refresh_box(
    in_refresh_box: &RefreshBoxWrapper,
    creation_height: u32,
    reward_decrement: u64,
) -> Result<ErgoBoxCandidate, RefrechActionError> {
    let mut builder = ErgoBoxCandidateBuilder::new(
        in_refresh_box.get_box().value,
        in_refresh_box.get_box().ergo_tree.clone(),
        creation_height,
    );
    builder.add_token(in_refresh_box.refresh_nft_token().clone());
    let reward_token = in_refresh_box.reward_token();
    let new_reward_token: Token = (
        reward_token.token_id,
        reward_token
            .amount
            .checked_sub(&reward_decrement.try_into().unwrap())
            .unwrap(),
    )
        .into();
    builder.add_token(new_reward_token);
    builder.build().map_err(Into::into)
}

fn build_out_oracle_boxes(
    valid_oracle_boxes: &Vec<&dyn OracleBox>,
    creation_height: u32,
) -> Result<Vec<ErgoBoxCandidate>, RefrechActionError> {
    valid_oracle_boxes
        .iter()
        .map(|in_ob| {
            let mut builder = ErgoBoxCandidateBuilder::new(
                in_ob.get_box().value,
                in_ob.get_box().ergo_tree.clone(),
                creation_height,
            );
            builder.set_register_value(R4, in_ob.public_key().into());
            builder.add_token(in_ob.oracle_token().clone());
            let mut reward_token_new = in_ob.reward_token();
            reward_token_new.amount = reward_token_new
                .amount
                .checked_add(&1u64.try_into().unwrap())
                .unwrap();
            builder.add_token(reward_token_new.clone());
            builder.build().map_err(Into::into)
        })
        .collect::<Result<Vec<ErgoBoxCandidate>, RefrechActionError>>()
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use std::convert::TryInto;

    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::chain::transaction::TxIoVec;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenId;
    use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
    use ergo_lib::ergotree_ir::mir::constant::Constant;
    use ergo_lib::ergotree_ir::mir::expr::Expr;
    use ergo_lib::ergotree_ir::sigma_protocol::dlog_group::EcPoint;
    use ergo_lib::ergotree_ir::sigma_protocol::sigma_boolean::ProveDlog;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use ergo_node_interface::node_interface::NodeError;
    use sigma_test_util::force_any_val;

    use crate::box_kind::OracleBoxWrapper;
    use crate::box_kind::PoolBoxWrapper;
    use crate::box_kind::RefreshBoxWrapper;
    use crate::contracts::oracle::OracleContract;
    use crate::contracts::pool::PoolContract;
    use crate::oracle_state::StageError;

    use super::*;

    #[derive(Clone)]
    struct RefreshBoxMock {
        refresh_box: RefreshBoxWrapper,
    }

    #[derive(Clone)]
    struct PoolBoxMock {
        pool_box: PoolBoxWrapper,
    }

    impl PoolBoxSource for PoolBoxMock {
        fn get_pool_box(&self) -> std::result::Result<PoolBoxWrapper, StageError> {
            Ok(self.pool_box.clone())
        }
    }

    impl RefreshBoxSource for RefreshBoxMock {
        fn get_refresh_box(&self) -> std::result::Result<RefreshBoxWrapper, StageError> {
            Ok(self.refresh_box.clone())
        }
    }

    #[derive(Clone)]
    struct DatapointStageMock {
        datapoints: Vec<OracleBoxWrapper>,
    }

    impl DatapointBoxesSource for DatapointStageMock {
        fn get_oracle_datapoint_boxes(
            &self,
        ) -> std::result::Result<Vec<OracleBoxWrapper>, StageError> {
            Ok(self.datapoints.clone())
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

    fn make_refresh_box(
        refresh_nft: &TokenId,
        reward_token: Token,
        value: BoxValue,
        creation_height: u32,
    ) -> RefreshBoxWrapper {
        let tokens = vec![
            Token::from((refresh_nft.clone(), 1u64.try_into().unwrap())),
            reward_token,
        ]
        .try_into()
        .unwrap();
        ErgoBox::new(
            value,
            RefreshContract::new().ergo_tree(),
            Some(tokens),
            NonMandatoryRegisters::empty(),
            creation_height,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
        .try_into()
        .unwrap()
    }

    fn make_pool_box(
        datapoint: i64,
        epoch_counter: i32,
        pool_nft_token_id: TokenId,
        value: BoxValue,
        creation_height: u32,
    ) -> PoolBoxWrapper {
        let tokens = [Token::from((
            pool_nft_token_id.clone(),
            1u64.try_into().unwrap(),
        ))]
        .into();
        ErgoBox::new(
            value,
            PoolContract::new().ergo_tree(),
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
        .try_into()
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
            OracleContract::new().ergo_tree(),
            Some(tokens),
            NonMandatoryRegisters::new(
                vec![
                    (NonMandatoryRegisterId::R4, Constant::from(pub_key)),
                    (NonMandatoryRegisterId::R5, Constant::from(epoch_counter)),
                    (NonMandatoryRegisterId::R6, Constant::from(datapoint)),
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

    fn make_datapoint_boxes(
        pub_keys: Vec<EcPoint>,
        datapoints: Vec<i64>,
        epoch_counter: i32,
        oracle_token_id: TokenId,
        reward_token: Token,
        value: BoxValue,
        creation_height: u32,
    ) -> Vec<OracleBoxWrapper> {
        datapoints
            .into_iter()
            .zip(pub_keys)
            .map(|(datapoint, pub_key)| {
                make_datapoint_box(
                    pub_key.clone(),
                    datapoint,
                    epoch_counter,
                    oracle_token_id.clone(),
                    reward_token.clone(),
                    value,
                    creation_height,
                )
                .try_into()
                .unwrap()
            })
            .collect()
    }

    fn make_wallet_unspent_box(pub_key: ProveDlog, value: BoxValue) -> ErgoBox {
        let c: Constant = pub_key.into();
        let expr: Expr = c.into();
        ErgoBox::new(
            value,
            ErgoTree::try_from(expr).unwrap(),
            None,
            NonMandatoryRegisters::empty(),
            1,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()
    }

    fn find_input_boxes(
        tx: UnsignedTransaction,
        available_boxes: Vec<ErgoBox>,
    ) -> TxIoVec<ErgoBox> {
        tx.inputs.mapped(|i| {
            available_boxes
                .clone()
                .into_iter()
                .find(|b| b.box_id() == i.box_id)
                .unwrap()
        })
    }

    #[test]
    fn test_refresh_pool() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let refresh_contract = RefreshContract::new();
        let pool_contract = PoolContract::new();
        let reward_token_id =
            TokenId::from_base64("RytLYlBlU2hWbVlxM3Q2dzl6JEMmRilKQE1jUWZUalc=").unwrap();
        let pool_nft_token_id = refresh_contract.pool_nft_token_id();
        let refresh_nft = pool_contract.refresh_nft_token_id();
        dbg!(&reward_token_id);
        let in_refresh_box = make_refresh_box(
            &refresh_nft,
            Token::from((reward_token_id.clone(), 100u64.try_into().unwrap())).clone(),
            BoxValue::SAFE_USER_MIN,
            height - 32,
        );
        let in_pool_box = make_pool_box(
            200,
            1,
            pool_nft_token_id,
            BoxValue::SAFE_USER_MIN,
            height - 32, // from previous epoch
        );
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let oracle_pub_key = secret.public_image().h;

        let oracle_pub_keys = vec![
            *oracle_pub_key,
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
        ];
        let in_oracle_boxes = make_datapoint_boxes(
            oracle_pub_keys,
            // TODO: filtering out outliers does not work
            vec![195, 196, 197, 198, 199, 200],
            1,
            refresh_contract.oracle_nft_token_id(),
            Token::from((reward_token_id, 5u64.try_into().unwrap())),
            BoxValue::SAFE_USER_MIN.checked_mul_u32(100).unwrap(),
            height - 9,
        );

        let pool_box_mock = PoolBoxMock {
            pool_box: in_pool_box,
        };
        let refresh_box_mock = RefreshBoxMock {
            refresh_box: in_refresh_box,
        };

        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();
        let datapoint_stage_mock = DatapointStageMock {
            datapoints: in_oracle_boxes.clone(),
        };

        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BoxValue::SAFE_USER_MIN.checked_mul_u32(10000).unwrap(),
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };
        let action = build_refresh_action(
            &pool_box_mock,
            &refresh_box_mock,
            &datapoint_stage_mock,
            &wallet_mock,
            height,
            change_address,
        )
        .unwrap();

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box(),
            refresh_box_mock.get_refresh_box().unwrap().get_box(),
        ];
        let mut in_oracle_boxes_raw: Vec<ErgoBox> =
            in_oracle_boxes.into_iter().map(Into::into).collect();
        possible_input_boxes.append(&mut in_oracle_boxes_raw);
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes),
            None,
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }
}
