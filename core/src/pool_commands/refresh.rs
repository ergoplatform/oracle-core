use crate::actions::RefreshAction;
use crate::box_kind::make_collected_oracle_box_candidate;
use crate::box_kind::make_pool_box_candidate;
use crate::box_kind::make_refresh_box_candidate;
use crate::box_kind::OracleBox;
use crate::box_kind::OracleBoxWrapper;
use crate::box_kind::PoolBox;
use crate::box_kind::PoolBoxWrapper;
use crate::box_kind::RefreshBox;
use crate::box_kind::RefreshBoxWrapper;
use crate::oracle_state::DatapointBoxesSource;
use crate::oracle_state::PoolBoxSource;
use crate::oracle_state::RefreshBoxSource;
use crate::oracle_state::StageError;
use crate::wallet::WalletDataSource;

use derive_more::From;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_interpreter::sigma_protocol::prover::ContextExtension;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
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

#[allow(clippy::too_many_arguments)]
pub fn build_refresh_action(
    pool_box_source: &dyn PoolBoxSource,
    refresh_box_source: &dyn RefreshBoxSource,
    datapoint_stage_src: &dyn DatapointBoxesSource,
    max_deviation_percent: u32,
    min_data_points: u32,
    wallet: &dyn WalletDataSource,
    height: u32,
    change_address: Address,
) -> Result<RefreshAction, RefrechActionError> {
    let tx_fee = BoxValue::SAFE_USER_MIN;

    let in_pool_box = pool_box_source.get_pool_box()?;
    let in_refresh_box = refresh_box_source.get_refresh_box()?;
    let mut in_oracle_boxes = datapoint_stage_src.get_oracle_datapoint_boxes()?;
    let deviation_range = max_deviation_percent;
    in_oracle_boxes.sort_by_key(|b| b.rate());
    let valid_in_oracle_boxes_datapoints = filtered_oracle_boxes(
        in_oracle_boxes.iter().map(|b| b.rate()).collect(),
        deviation_range,
    )?;
    let valid_in_oracle_boxes = in_oracle_boxes
        .into_iter()
        .filter(|b| valid_in_oracle_boxes_datapoints.contains(&b.rate()))
        .collect::<Vec<_>>();
    if (valid_in_oracle_boxes.len() as u32) < min_data_points {
        return Err(RefrechActionError::FailedToReachConsensus {
            found: valid_in_oracle_boxes.len() as u32,
            expected: min_data_points,
        });
    }
    let rate = calc_pool_rate(valid_in_oracle_boxes.iter().map(|b| b.rate()).collect());
    let reward_decrement = valid_in_oracle_boxes.len() as u64 * 2;
    let out_pool_box = build_out_pool_box(&in_pool_box, height, rate, reward_decrement)?;
    let out_refresh_box = build_out_refresh_box(&in_refresh_box, height)?;
    let mut out_oracle_boxes = build_out_oracle_boxes(&valid_in_oracle_boxes, height)?;

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;

    let mut input_boxes = vec![
        in_pool_box.get_box().clone(),
        in_refresh_box.get_box().clone(),
    ];
    let mut valid_in_oracle_raw_boxes = valid_in_oracle_boxes
        .clone()
        .into_iter()
        .map(|ob| ob.get_box().clone())
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
    oracle_boxes: Vec<u64>,
    deviation_range: u32,
) -> Result<Vec<u64>, RefrechActionError> {
    let mut successful_boxes = oracle_boxes.clone();
    // The min oracle box's rate must be within deviation_range(5%) of that of the max
    while !deviation_check(deviation_range, &successful_boxes) {
        // Removing largest deviation outlier
        successful_boxes = remove_largest_local_deviation_datapoint(successful_boxes)?;
    }
    dbg!(&successful_boxes);
    Ok(successful_boxes)
}

fn deviation_check(max_deviation_range: u32, datapoint_boxes: &Vec<u64>) -> bool {
    let min_datapoint = datapoint_boxes.iter().min().unwrap();
    let max_datapoint = datapoint_boxes.iter().max().unwrap();
    let deviation_delta = max_datapoint * (max_deviation_range as u64) / 100;
    max_datapoint - min_datapoint <= deviation_delta
}

/// Finds whether the max or the min value in a list of sorted Datapoint boxes
/// deviates more compared to their adjacted datapoint, and then removes
/// said datapoint which deviates further.
fn remove_largest_local_deviation_datapoint(
    datapoint_boxes: Vec<u64>,
) -> Result<Vec<u64>, RefrechActionError> {
    // Check if sufficient number of datapoint boxes to start removing
    if datapoint_boxes.len() <= 2 {
        Err(RefrechActionError::NotEnoughDatapoints)
    } else {
        let mean = (datapoint_boxes.iter().sum::<u64>() as f32) / datapoint_boxes.len() as f32;
        let min_datapoint = *datapoint_boxes.iter().min().unwrap();
        let max_datapoint = *datapoint_boxes.iter().max().unwrap();
        let front_deviation = max_datapoint as f32 - mean;
        let back_deviation = mean - min_datapoint as f32;
        if front_deviation >= back_deviation {
            // Remove largest datapoint if front deviation is greater
            Ok(datapoint_boxes
                .into_iter()
                .filter(|dp| *dp != max_datapoint)
                .collect())
        } else {
            // Remove smallest datapoint if back deviation is greater
            Ok(datapoint_boxes
                .into_iter()
                .filter(|dp| *dp != min_datapoint)
                .collect())
        }
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
    reward_decrement: u64,
) -> Result<ErgoBoxCandidate, RefrechActionError> {
    let new_epoch_counter: i32 = (in_pool_box.epoch_counter() + 1) as i32;
    let reward_token = in_pool_box.reward_token();
    let new_reward_token: Token = (
        reward_token.token_id,
        reward_token
            .amount
            .checked_sub(&reward_decrement.try_into().unwrap())
            .unwrap(),
    )
        .into();

    make_pool_box_candidate(
        in_pool_box.contract(),
        rate as i64,
        new_epoch_counter,
        in_pool_box.pool_nft_token().clone(),
        new_reward_token,
        in_pool_box.get_box().value,
        creation_height,
    )
    .map_err(Into::into)
}

fn build_out_refresh_box(
    in_refresh_box: &RefreshBoxWrapper,
    creation_height: u32,
) -> Result<ErgoBoxCandidate, RefrechActionError> {
    make_refresh_box_candidate(
        in_refresh_box.contract(),
        in_refresh_box.refresh_nft_token(),
        in_refresh_box.get_box().value,
        creation_height,
    )
    .map_err(Into::into)
}

fn build_out_oracle_boxes(
    valid_oracle_boxes: &Vec<OracleBoxWrapper>,
    creation_height: u32,
) -> Result<Vec<ErgoBoxCandidate>, RefrechActionError> {
    valid_oracle_boxes
        .iter()
        .map(|in_ob| {
            let mut reward_token_new = in_ob.reward_token();
            reward_token_new.amount = reward_token_new
                .amount
                .checked_add(&1u64.try_into().unwrap())
                .unwrap();
            make_collected_oracle_box_candidate(
                in_ob.contract(),
                in_ob.public_key(),
                in_ob.oracle_token(),
                reward_token_new,
                in_ob.get_box().value,
                creation_height,
            )
            .map_err(Into::into)
        })
        .collect::<Result<Vec<ErgoBoxCandidate>, RefrechActionError>>()
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::ergo_chain_types::EcPoint;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::ergotree_ir::chain::token::TokenId;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use sigma_test_util::force_any_val;

    use crate::box_kind::OracleBoxWrapper;
    use crate::box_kind::OracleBoxWrapperInputs;
    use crate::box_kind::RefreshBoxWrapper;
    use crate::box_kind::RefreshBoxWrapperInputs;
    use crate::contracts::oracle::OracleContractParameters;
    use crate::contracts::pool::PoolContractParameters;
    use crate::contracts::refresh::RefreshContract;
    use crate::contracts::refresh::RefreshContractParameters;
    use crate::oracle_config::TokenIds;
    use crate::oracle_state::StageError;
    use crate::pool_commands::test_utils::generate_token_ids;
    use crate::pool_commands::test_utils::{
        find_input_boxes, make_datapoint_box, make_pool_box, make_wallet_unspent_box, PoolBoxMock,
        WalletDataMock,
    };

    use super::*;

    #[derive(Clone)]
    struct RefreshBoxMock {
        refresh_box: RefreshBoxWrapper,
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

    fn make_refresh_box(
        value: BoxValue,
        inputs: RefreshBoxWrapperInputs,
        creation_height: u32,
    ) -> RefreshBoxWrapper {
        let tokens = vec![Token::from((
            inputs.refresh_nft_token_id.clone(),
            1u64.try_into().unwrap(),
        ))]
        .try_into()
        .unwrap();
        RefreshBoxWrapper::new(
            ErgoBox::new(
                value,
                RefreshContract::new(inputs.into()).unwrap().ergo_tree(),
                Some(tokens),
                NonMandatoryRegisters::empty(),
                creation_height,
                force_any_val::<TxId>(),
                0,
            )
            .unwrap(),
            inputs,
        )
        .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn make_datapoint_boxes(
        pub_keys: Vec<EcPoint>,
        datapoints: Vec<i64>,
        epoch_counter: i32,
        value: BoxValue,
        creation_height: u32,
        oracle_contract_parameters: &OracleContractParameters,
        token_ids: &TokenIds,
    ) -> Vec<OracleBoxWrapper> {
        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::from((oracle_contract_parameters, token_ids));
        datapoints
            .into_iter()
            .zip(pub_keys)
            .map(|(datapoint, pub_key)| {
                (
                    make_datapoint_box(
                        pub_key.clone(),
                        datapoint,
                        epoch_counter,
                        token_ids,
                        value,
                        creation_height,
                    ),
                    oracle_box_wrapper_inputs,
                )
                    .try_into()
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn test_refresh_pool() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let reward_token_id = force_any_val::<TokenId>();
        dbg!(&reward_token_id);
        let pool_contract_parameters = PoolContractParameters::default();
        let oracle_contract_parameters = OracleContractParameters::default();
        let refresh_contract_parameters = RefreshContractParameters::default();
        let token_ids = generate_token_ids();

        let inputs = RefreshBoxWrapperInputs {
            contract_parameters: &refresh_contract_parameters,
            refresh_nft_token_id: &token_ids.refresh_nft_token_id,
            oracle_token_id: &token_ids.oracle_token_id,
            pool_nft_token_id: &token_ids.pool_nft_token_id,
        };
        let in_refresh_box = make_refresh_box(BoxValue::SAFE_USER_MIN, inputs, height - 32);
        let in_pool_box = make_pool_box(
            200,
            1,
            BoxValue::SAFE_USER_MIN,
            height - 32, // from previous epoch
            &pool_contract_parameters,
            &token_ids,
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
            vec![194, 70, 196, 197, 198, 200],
            1,
            BoxValue::SAFE_USER_MIN.checked_mul_u32(100).unwrap(),
            height - 9,
            &oracle_contract_parameters,
            &token_ids,
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
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
        };
        let action = build_refresh_action(
            &pool_box_mock,
            &refresh_box_mock,
            &datapoint_stage_mock,
            5,
            4,
            &wallet_mock,
            height,
            change_address,
        )
        .unwrap();

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box().clone(),
            refresh_box_mock
                .get_refresh_box()
                .unwrap()
                .get_box()
                .clone(),
        ];
        let mut in_oracle_boxes_raw: Vec<ErgoBox> =
            in_oracle_boxes.into_iter().map(Into::into).collect();
        possible_input_boxes.append(&mut in_oracle_boxes_raw);
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();
    }

    #[test]
    fn test_oracle_deviation_check() {
        assert_eq!(
            filtered_oracle_boxes(vec![95, 96, 97, 98, 99, 200], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes(vec![70, 95, 96, 97, 98, 99, 200], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes(vec![70, 95, 96, 97, 98, 99], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes(vec![70, 70, 95, 96, 97, 98, 99], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes(vec![95, 96, 97, 98, 99, 200, 200], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
    }
}
