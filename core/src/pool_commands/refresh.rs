use crate::action_report::RefreshActionReport;
use crate::actions::RefreshAction;
use crate::box_kind::make_collected_oracle_box_candidate;
use crate::box_kind::make_pool_box_candidate;
use crate::box_kind::make_refresh_box_candidate;
use crate::box_kind::PoolBox;
use crate::box_kind::PoolBoxWrapper;
use crate::box_kind::PostedOracleBox;
use crate::box_kind::RefreshBox;
use crate::box_kind::RefreshBoxWrapper;
use crate::oracle_config::BASE_FEE;
use crate::oracle_state::BuybackBoxSource;
use crate::oracle_state::DataSourceError;
use crate::oracle_state::PoolBoxSource;
use crate::oracle_state::PostedDatapointBoxesSource;
use crate::oracle_state::RefreshBoxSource;
use crate::oracle_types::BlockHeight;
use crate::oracle_types::EpochCounter;
use crate::oracle_types::MinDatapoints;
use crate::oracle_types::Rate;
use crate::spec_token::RewardTokenId;
use crate::spec_token::SpecToken;
use crate::wallet::WalletDataError;
use crate::wallet::WalletDataSource;

use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_interpreter::sigma_protocol::prover::ContextExtension;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBoxCandidate;
use ergo_lib::ergotree_ir::chain::token::TokenAmount;
use ergo_lib::wallet::box_selector::BoxSelection;
use ergo_lib::wallet::box_selector::BoxSelector;
use ergo_lib::wallet::box_selector::BoxSelectorError;
use ergo_lib::wallet::box_selector::SimpleBoxSelector;
use ergo_lib::wallet::tx_builder::TxBuilder;
use ergo_lib::wallet::tx_builder::TxBuilderError;
use thiserror::Error;

use std::convert::TryInto;

#[derive(Debug, Error)]
pub enum RefreshActionError {
    #[error("Refresh failed, not enough datapoints. The minimum number of datapoints within the deviation range: required minumum {expected}, found {found_num} from public keys {found_public_keys:?},")]
    FailedToReachConsensus {
        found_public_keys: Vec<EcPoint>,
        found_num: i32,
        expected: i32,
    },
    #[error("Not enough datapoints left during the removal of the outliers")]
    NotEnoughDatapoints,
    #[error("data source error: {0}")]
    DataSourceError(#[from] DataSourceError),
    #[error("WalletData error: {0}")]
    WalletData(#[from] WalletDataError),
    #[error("box selector error: {0}")]
    BoxSelectorError(#[from] BoxSelectorError),
    #[error("tx builder error: {0}")]
    TxBuilderError(#[from] TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilderError(#[from] ErgoBoxCandidateBuilderError),
    #[error("failed to found my own oracle box in the filtered posted oracle boxes")]
    MyOracleBoxNoFound,
}

#[allow(clippy::too_many_arguments)]
pub fn build_refresh_action(
    pool_box_source: &dyn PoolBoxSource,
    refresh_box_source: &dyn RefreshBoxSource,
    datapoint_src: &dyn PostedDatapointBoxesSource,
    max_deviation_percent: u32,
    min_data_points: MinDatapoints,
    wallet: &dyn WalletDataSource,
    height: BlockHeight,
    change_address: Address,
    my_oracle_pk: &EcPoint,
    buyback_box_source: Option<&dyn BuybackBoxSource>,
    dev_reward_ergo_tree_bytes: Option<String>,
) -> Result<(RefreshAction, RefreshActionReport), RefreshActionError> {
    let tx_fee = *BASE_FEE;
    let in_pool_box = pool_box_source.get_pool_box()?;
    let in_refresh_box = refresh_box_source.get_refresh_box()?;
    let min_start_height = height - in_refresh_box.contract().epoch_length();
    let in_pool_box_epoch_id = in_pool_box.epoch_counter();
    let mut in_oracle_boxes: Vec<PostedOracleBox> = datapoint_src
        .get_posted_datapoint_boxes()?
        .into_iter()
        .filter(|b| {
            b.get_box().creation_height > min_start_height.0
                && b.epoch_counter() == in_pool_box_epoch_id
        })
        .collect();
    // log::info!("Building refresh action {:?}", in_oracle_boxes);
    let deviation_range = max_deviation_percent;
    in_oracle_boxes.sort_by_key(|b| b.rate());
    let valid_in_oracle_boxes_datapoints = filtered_oracle_boxes_by_rate(
        in_oracle_boxes.iter().map(|b| b.rate()).collect(),
        deviation_range,
    )?;
    let valid_in_oracle_boxes = in_oracle_boxes
        .into_iter()
        .filter(|b| valid_in_oracle_boxes_datapoints.contains(&b.rate()))
        .collect::<Vec<_>>();
    if (valid_in_oracle_boxes.len() as i32) < min_data_points.0 {
        return Err(RefreshActionError::FailedToReachConsensus {
            found_num: valid_in_oracle_boxes.len() as i32,
            expected: min_data_points.0,
            found_public_keys: valid_in_oracle_boxes
                .iter()
                .map(|b| b.public_key())
                .collect(),
        });
    }
    let rate = calc_pool_rate(valid_in_oracle_boxes.iter().map(|b| b.rate()).collect());
    let reward_decrement = valid_in_oracle_boxes.len() as u64 * 2;
    let out_refresh_box = build_out_refresh_box(&in_refresh_box, height)?;
    let mut out_oracle_boxes =
        build_out_oracle_boxes(&valid_in_oracle_boxes, height, my_oracle_pk)?;

    let in_buyback_box_opt = buyback_box_source
        .map(|s| s.get_buyback_box())
        .transpose()?
        .flatten();

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    let box_selector = SimpleBoxSelector::new();
    let selection = box_selector.select(unspent_boxes, tx_fee, &[])?;

    let mut input_boxes = vec![
        in_pool_box.get_box().clone(),
        in_refresh_box.get_box().clone(),
    ];
    let my_input_oracle_box_index: i32 = valid_in_oracle_boxes
        .iter()
        .position(|b| &b.public_key() == my_oracle_pk)
        .ok_or(RefreshActionError::MyOracleBoxNoFound)?
        as i32;

    let mut valid_in_oracle_raw_boxes = valid_in_oracle_boxes
        .clone()
        .into_iter()
        .map(|ob| ob.get_box().clone())
        .collect();
    log::info!(
        "Refresh: Found {} valid oracle boxes, next pool rate is {rate}",
        valid_in_oracle_boxes.len()
    );

    let out_pool_box = build_out_pool_box(&in_pool_box, height, rate, reward_decrement, None)?;
    let mut output_candidates = vec![out_pool_box, out_refresh_box];
    if let Some(buyback_box) = in_buyback_box_opt {
        log::debug!("Found buyback box id {:?}", buyback_box.get_box().box_id());
        if let Some(buyback_reward_token) = buyback_box.reward_token() {
            log::debug!(
                "Found reward tokens in buyback box and including it in the tx. Amount: {:?}",
                buyback_reward_token.amount
            );
            input_boxes.push(buyback_box.get_box().clone());
            let out_pool_box_w_buyback_rewards = build_out_pool_box(
                &in_pool_box,
                height,
                rate,
                reward_decrement,
                Some(buyback_reward_token.amount),
            )?;
            let out_buyback_box = buyback_box.new_without_reward_token();
            output_candidates.remove(0);
            output_candidates.insert(0, out_pool_box_w_buyback_rewards);
            // should be at index 2 (checked in the contract of the buyback input box)
            output_candidates.push(out_buyback_box);
        } else {
            log::debug!("No reward tokens in buyback box");
        }
    };
    input_boxes.append(&mut valid_in_oracle_raw_boxes);
    input_boxes.append(selection.boxes.as_vec().clone().as_mut());
    output_candidates.append(&mut out_oracle_boxes);

    match dev_reward_ergo_tree_bytes {
        // The division was valid
        Some(tbs) => {
            let tb = base16::decode(tbs.as_str()).unwrap();
            let t: ErgoTree = ErgoTree::sigma_parse_bytes(tb.as_slice()).unwrap();
            let mut builder = ErgoBoxCandidateBuilder::new(*BASE_FEE, t, height.0);
            let mut dev_reward_token = in_pool_box.reward_token();
            dev_reward_token.amount = TokenAmount::try_from((valid_in_oracle_boxes.len() as u64) - (1 as u64)).unwrap();
            builder.add_token(dev_reward_token.into());
            let devout = builder.build().unwrap();
            output_candidates.push(devout);
        },

        // The division was invalid
        None    => {}
    }

    let box_selection = BoxSelection {
        boxes: input_boxes.clone().try_into().unwrap(),
        change_boxes: selection.change_boxes,
    };
    let mut b = TxBuilder::new(
        box_selection,
        output_candidates,
        height.0,
        tx_fee,
        change_address,
    );
    let in_refresh_box_ctx_ext = ContextExtension {
        values: vec![(0, my_input_oracle_box_index.into())]
            .into_iter()
            .collect(),
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
    let report = RefreshActionReport {
        oracle_boxes_collected: valid_in_oracle_boxes
            .iter()
            .map(|b| b.public_key())
            .collect(),
    };
    Ok((RefreshAction { tx }, report))
}

fn filtered_oracle_boxes_by_rate<T>(
    oracle_boxes: Vec<T>,
    deviation_range: u32,
) -> Result<Vec<Rate>, RefreshActionError>
where
    T: Into<Rate>,
    T: Clone,
{
    let oracle_boxes = oracle_boxes
        .into_iter()
        .map(|b| b.into())
        .collect::<Vec<_>>();
    if oracle_boxes.is_empty() {
        return Ok(oracle_boxes);
    }
    let mut successful_boxes = oracle_boxes.clone();
    // The min oracle box's rate must be within deviation_range(5%) of that of the max
    while !deviation_check(deviation_range, successful_boxes.clone()) {
        // Removing largest deviation outlier
        successful_boxes = remove_largest_local_deviation_datapoint(successful_boxes)?;
    }
    // dbg!(&successful_boxes);
    Ok(successful_boxes)
}

fn deviation_check(max_deviation_range: u32, datapoint_boxes: Vec<Rate>) -> bool {
    let min_datapoint = datapoint_boxes.clone().into_iter().min().unwrap();
    let max_datapoint = datapoint_boxes.into_iter().max().unwrap();
    let deviation_delta = max_datapoint * (max_deviation_range as i64) / 100;
    max_datapoint - min_datapoint <= deviation_delta
}

/// Finds whether the max or the min value in a list of sorted Datapoint boxes
/// deviates more compared to their adjacted datapoint, and then removes
/// said datapoint which deviates further.
fn remove_largest_local_deviation_datapoint(
    datapoint_boxes: Vec<Rate>,
) -> Result<Vec<Rate>, RefreshActionError> {
    // Check if sufficient number of datapoint boxes to start removing
    if datapoint_boxes.len() <= 2 {
        Err(RefreshActionError::NotEnoughDatapoints)
    } else {
        let mean = datapoint_boxes.clone().into_iter().sum::<Rate>().as_f32()
            / datapoint_boxes.len() as f32;
        let min_datapoint = *datapoint_boxes.iter().min().unwrap();
        let max_datapoint = *datapoint_boxes.iter().max().unwrap();
        let front_deviation = max_datapoint.as_f32() - mean;
        let back_deviation = mean - min_datapoint.as_f32();
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

fn calc_pool_rate(oracle_boxes_rates: Vec<Rate>) -> Rate {
    let datapoints_sum: i64 = oracle_boxes_rates.clone().into_iter().map(i64::from).sum();
    (datapoints_sum / oracle_boxes_rates.len() as i64).into()
}

fn build_out_pool_box(
    in_pool_box: &PoolBoxWrapper,
    creation_height: BlockHeight,
    rate: Rate,
    reward_decrement: u64,
    buyback_reward: Option<TokenAmount>,
) -> Result<ErgoBoxCandidate, RefreshActionError> {
    let new_epoch_counter = EpochCounter(in_pool_box.epoch_counter().0 + 1);
    let reward_token = in_pool_box.reward_token();
    let decremented = reward_token
        .amount
        .checked_sub(&reward_decrement.try_into().unwrap())
        .unwrap();
    let new_reward_amount = if let Some(buyback_reward) = buyback_reward {
        decremented.checked_add(&buyback_reward).unwrap()
    } else {
        decremented
    };
    let new_reward_token: SpecToken<RewardTokenId> = SpecToken {
        token_id: reward_token.token_id,
        amount: new_reward_amount,
    };

    make_pool_box_candidate(
        in_pool_box.contract(),
        rate.into(),
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
    creation_height: BlockHeight,
) -> Result<ErgoBoxCandidate, RefreshActionError> {
    make_refresh_box_candidate(
        in_refresh_box.contract(),
        in_refresh_box.refresh_nft_token(),
        in_refresh_box.get_box().value,
        creation_height,
    )
    .map_err(Into::into)
}

fn build_out_oracle_boxes(
    valid_oracle_boxes: &Vec<PostedOracleBox>,
    creation_height: BlockHeight,
    my_public_key: &EcPoint,
) -> Result<Vec<ErgoBoxCandidate>, RefreshActionError> {
    valid_oracle_boxes
        .iter()
        .map(|in_ob| {
            let mut reward_token_new = in_ob.reward_token();
            reward_token_new.amount = if &in_ob.public_key() == my_public_key {
                // 2 reward tokens per collected oracle box goes to the collector
                let increment: TokenAmount = (2 as u64).try_into().unwrap();
                reward_token_new.amount.checked_add(&increment).unwrap()
            } else {
                reward_token_new
                    .amount
                    .checked_add(&1u64.try_into().unwrap())
                    .unwrap()
            };
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
        .collect::<Result<Vec<ErgoBoxCandidate>, RefreshActionError>>()
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use std::convert::TryInto;
    use std::vec;

    use ergo_lib::chain::ergo_state_context::ErgoStateContext;
    use ergo_lib::chain::transaction::TxId;
    use ergo_lib::ergo_chain_types::EcPoint;
    use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
    use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
    use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
    use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
    use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
    use ergo_lib::ergotree_ir::chain::token::Token;
    use ergo_lib::wallet::signing::TransactionContext;
    use ergo_lib::wallet::Wallet;
    use sigma_test_util::force_any_val;

    use crate::box_kind::BuybackBoxWrapper;
    use crate::box_kind::OracleBoxWrapperInputs;
    use crate::box_kind::PostedOracleBox;
    use crate::box_kind::RefreshBoxWrapper;
    use crate::box_kind::RefreshBoxWrapperInputs;
    use crate::contracts::oracle::OracleContractParameters;
    use crate::contracts::pool::PoolContractParameters;
    use crate::contracts::refresh::RefreshContract;
    use crate::contracts::refresh::RefreshContractInputs;
    use crate::contracts::refresh::RefreshContractParameters;
    use crate::oracle_config::BASE_FEE;
    use crate::oracle_state::DataSourceError;
    use crate::oracle_types::EpochLength;
    use crate::pool_commands::test_utils::generate_token_ids;
    use crate::pool_commands::test_utils::BuybackBoxSourceMock;
    use crate::pool_commands::test_utils::{
        find_input_boxes, make_datapoint_box, make_pool_box, make_wallet_unspent_box, PoolBoxMock,
        WalletDataMock,
    };
    use crate::pool_config::TokenIds;
    use crate::spec_token::TokenIdKind;

    use super::*;

    #[derive(Clone)]
    struct RefreshBoxMock {
        refresh_box: RefreshBoxWrapper,
    }

    impl RefreshBoxSource for RefreshBoxMock {
        fn get_refresh_box(&self) -> std::result::Result<RefreshBoxWrapper, DataSourceError> {
            Ok(self.refresh_box.clone())
        }
    }

    #[derive(Clone)]
    struct DatapointSourceMock {
        datapoints: Vec<PostedOracleBox>,
    }

    impl PostedDatapointBoxesSource for DatapointSourceMock {
        fn get_posted_datapoint_boxes(
            &self,
        ) -> std::result::Result<Vec<PostedOracleBox>, DataSourceError> {
            Ok(self.datapoints.clone())
        }
    }

    fn make_refresh_box(
        value: BoxValue,
        inputs: &RefreshBoxWrapperInputs,
        creation_height: BlockHeight,
    ) -> RefreshBoxWrapper {
        let tokens = vec![Token::from((
            inputs.refresh_nft_token_id.token_id(),
            1u64.try_into().unwrap(),
        ))]
        .try_into()
        .unwrap();
        RefreshBoxWrapper::new(
            ErgoBox::new(
                value,
                RefreshContract::checked_load(&inputs.contract_inputs)
                    .unwrap()
                    .ergo_tree(),
                Some(tokens),
                NonMandatoryRegisters::empty(),
                creation_height.0,
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
        epoch_counter: EpochCounter,
        value: BoxValue,
        creation_height: BlockHeight,
        oracle_contract_parameters: &OracleContractParameters,
        token_ids: &TokenIds,
    ) -> Vec<PostedOracleBox> {
        let oracle_box_wrapper_inputs =
            OracleBoxWrapperInputs::try_from((oracle_contract_parameters.clone(), token_ids))
                .unwrap();
        datapoints
            .into_iter()
            .zip(pub_keys)
            .map(|(datapoint, pub_key)| {
                PostedOracleBox::new(
                    make_datapoint_box(
                        pub_key.clone(),
                        datapoint,
                        epoch_counter,
                        token_ids,
                        value,
                        creation_height,
                        100,
                    ),
                    &oracle_box_wrapper_inputs,
                )
                .unwrap()
            })
            .collect()
    }

    #[test]
    fn test_refresh_pool() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = BlockHeight(ctx.pre_header.height);
        let pool_contract_parameters = PoolContractParameters::default();
        let oracle_contract_parameters = OracleContractParameters::default();
        let refresh_contract_parameters = RefreshContractParameters::default();
        let token_ids = generate_token_ids();
        dbg!(&token_ids);

        let refresh_contract_inputs = RefreshContractInputs::build_with(
            refresh_contract_parameters,
            token_ids.oracle_token_id.clone(),
            token_ids.pool_nft_token_id.clone(),
        )
        .unwrap();

        let inputs = RefreshBoxWrapperInputs {
            refresh_nft_token_id: token_ids.refresh_nft_token_id.clone(),
            contract_inputs: refresh_contract_inputs,
        };
        let pool_box_epoch_id = EpochCounter(1);
        let in_refresh_box = make_refresh_box(*BASE_FEE, &inputs, height - EpochLength(32));
        let in_pool_box = make_pool_box(
            200,
            pool_box_epoch_id,
            *BASE_FEE,
            height - EpochLength(32), // from previous epoch
            &pool_contract_parameters,
            &token_ids,
        );
        let secret = force_any_val::<DlogProverInput>();
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let oracle_pub_key = secret.public_image().h;

        let oracle_pub_keys = vec![
            *oracle_pub_key.clone(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
            force_any_val::<EcPoint>(),
        ];

        let in_oracle_boxes = make_datapoint_boxes(
            oracle_pub_keys.clone(),
            vec![199, 70, 196, 197, 198, 200],
            pool_box_epoch_id,
            BASE_FEE.checked_mul_u32(100).unwrap(),
            height - EpochLength(9),
            &oracle_contract_parameters,
            &token_ids,
        );
        let mut in_oracle_boxes_raw: Vec<ErgoBox> = in_oracle_boxes
            .clone()
            .into_iter()
            .map(Into::into)
            .collect();

        let pool_box_mock = PoolBoxMock {
            pool_box: in_pool_box,
        };
        let refresh_box_mock = RefreshBoxMock {
            refresh_box: in_refresh_box,
        };

        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();
        let wallet_unspent_box = make_wallet_unspent_box(
            secret.public_image(),
            BASE_FEE.checked_mul_u32(10000).unwrap(),
            None,
        );
        let wallet_mock = WalletDataMock {
            unspent_boxes: vec![wallet_unspent_box],
            change_address: change_address.clone(),
        };

        let (action, report) = build_refresh_action(
            &pool_box_mock,
            &refresh_box_mock,
            &(DatapointSourceMock {
                datapoints: in_oracle_boxes.clone(),
            }),
            5,
            MinDatapoints(4),
            &wallet_mock,
            height,
            change_address.address(),
            &oracle_pub_key,
            None,
            None,
        )
        .unwrap();

        assert_eq!(report.oracle_boxes_collected.len(), 5);

        let mut possible_input_boxes = vec![
            pool_box_mock.get_pool_box().unwrap().get_box().clone(),
            refresh_box_mock
                .get_refresh_box()
                .unwrap()
                .get_box()
                .clone(),
        ];
        possible_input_boxes.append(&mut in_oracle_boxes_raw);
        possible_input_boxes.append(&mut wallet_mock.get_unspent_wallet_boxes().unwrap());

        let tx_context = TransactionContext::new(
            action.tx.clone(),
            find_input_boxes(action.tx, possible_input_boxes),
            Vec::new(),
        )
        .unwrap();

        let _signed_tx = wallet.sign_transaction(tx_context, &ctx, None).unwrap();

        let wrong_epoch_id_datapoints_mock = DatapointSourceMock {
            datapoints: make_datapoint_boxes(
                oracle_pub_keys.clone(),
                vec![199, 70, 196, 197, 198, 200],
                EpochCounter(pool_box_epoch_id.0 + 1),
                BASE_FEE.checked_mul_u32(100).unwrap(),
                height - EpochLength(9),
                &oracle_contract_parameters,
                &token_ids,
            ),
        };
        let wrong_epoch_res = build_refresh_action(
            &pool_box_mock,
            &refresh_box_mock,
            &wrong_epoch_id_datapoints_mock,
            5,
            MinDatapoints(4),
            &wallet_mock,
            height,
            change_address.address(),
            &oracle_pub_key,
            None,
            None,
        );
        dbg!(&wrong_epoch_res);
        assert!(matches!(
            wrong_epoch_res.unwrap_err(),
            RefreshActionError::FailedToReachConsensus {
                found_public_keys: _,
                found_num: _,
                expected: _,
            }
        ));

        let buyback_token_id = force_any_val();

        let buyback_box = make_wallet_unspent_box(
            secret.public_image(),
            *BASE_FEE,
            Some(
                vec![
                    Token {
                        token_id: buyback_token_id,
                        amount: 1u64.try_into().unwrap(),
                    },
                    Token {
                        token_id: token_ids.reward_token_id.token_id(),
                        amount: 100u64.try_into().unwrap(),
                    },
                ]
                .try_into()
                .unwrap(),
            ),
        );

        let buyback_source = BuybackBoxSourceMock {
            buyback_box: BuybackBoxWrapper::new(buyback_box, token_ids.reward_token_id.clone()),
        };

        let (action_with_buyback, _) = build_refresh_action(
            &pool_box_mock,
            &refresh_box_mock,
            &(DatapointSourceMock {
                datapoints: in_oracle_boxes.clone(),
            }),
            5,
            MinDatapoints(4),
            &wallet_mock,
            height,
            change_address.address(),
            &oracle_pub_key,
            Some(&buyback_source),
            None,
        )
        .unwrap();

        assert_eq!(
            action_with_buyback
                .tx
                .output_candidates
                .get(2)
                .unwrap()
                .tokens
                .as_ref()
                .unwrap()
                .len(),
            1,
            "only one token should be in output buyback box"
        );
        assert_eq!(
            action_with_buyback
                .tx
                .output_candidates
                .get(2)
                .unwrap()
                .tokens
                .as_ref()
                .unwrap()
                .get(0)
                .unwrap()
                .token_id,
            buyback_token_id,
            "only buyback nft should be in output buyback box"
        );
        assert_eq!(
            action_with_buyback
                .tx
                .output_candidates
                .get(0)
                .unwrap()
                .tokens
                .as_ref()
                .unwrap()
                .get(1)
                .unwrap()
                .amount
                .as_u64(),
            &190,
            "reward tokens should be added to the pool box"
        )
    }

    #[test]
    fn test_oracle_deviation_check() {
        assert_eq!(
            filtered_oracle_boxes_by_rate(vec![95, 96, 97, 98, 99, 200], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes_by_rate(vec![70, 95, 96, 97, 98, 99, 200], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes_by_rate(vec![70, 95, 96, 97, 98, 99], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes_by_rate(vec![70, 70, 95, 96, 97, 98, 99], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
        assert_eq!(
            filtered_oracle_boxes_by_rate(vec![95, 96, 97, 98, 99, 200, 200], 5).unwrap(),
            vec![95, 96, 97, 98, 99]
        );
    }
}
