// use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
// use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;

// use crate::oracle_config::PoolParameters;

// use super::CollectionError;

// pub fn prepare_datapoints(
//     parameters: PoolParameters,
//     oracle_boxes: Vec<ErgoBox>,
//     live_epoch_id: u32,
// ) -> Result<Vec<ErgoBox>, CollectionError> {
//     let current_epoch_datapoint_boxes = current_epoch_boxes_filter(&oracle_boxes, live_epoch_id);
//     // Sort Datapoint boxes in decreasing order
//     let sorted_datapoint_boxes = sort_datapoint_boxes(&current_epoch_datapoint_boxes);
//     // Acquire the finalized oracle pool datapoint and the list of successful datapoint boxes which were within the deviation range
//     let (finalized_datapoint, successful_boxes) = finalize_datapoint(
//         &sorted_datapoint_boxes,
//         parameters.deviation_range as i64, // Make sure to change this to config #
//         parameters.consensus_num as i64,   // Make sure to change this to config #
//     )?;
//     Ok(successful_boxes)
// }

// /// Filters out Datapoint boxes that are not from the current epoch
// /// Also calls `valid_boxes_filter()` to remove invalid boxes.
// pub fn current_epoch_boxes_filter(
//     datapoint_boxes: &Vec<ErgoBox>,
//     live_epoch_id: u32,
// ) -> Vec<ErgoBox> {
//     let mut filtered_boxes = vec![];
//     let valid_boxes = valid_boxes_filter(datapoint_boxes);
//     for b in valid_boxes {
//         if let Ok(s) = unwrap_int(&b.get_register(NonMandatoryRegisterId::R5.into()).unwrap()) {
//             if s == live_epoch_id as i32 {
//                 filtered_boxes.push(b.clone());
//             }
//         }
//     }
//     filtered_boxes
// }

// /// Sort Datapoint boxes in decreasing order (from highest to lowest) based on Datapoint value.
// pub fn sort_datapoint_boxes(boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
//     let mut datapoint_boxes = boxes.clone();
//     datapoint_boxes.sort_by_key(|b| {
//         unwrap_long(&b.get_register(NonMandatoryRegisterId::R6.into()).unwrap()).unwrap_or(0)
//     });
//     datapoint_boxes.reverse();
//     datapoint_boxes
// }

// // Function which produces the finalized datapoint based on a list of `ErgoBox`es.
// /// If list of Datapoint boxes is outside of the deviation range then
// /// attempts to filter boxes until a list which is within deviation range
// /// is found.
// /// Returns the averaged datapoint and the filtered list of successful boxes.
// pub fn finalize_datapoint(
//     boxes: &Vec<ErgoBox>,
//     deviation_range: i64,
//     consensus_num: i64,
// ) -> Result<(u64, Vec<ErgoBox>), CollectionError> {
//     let mut successful_boxes = boxes.clone();
//     while !deviation_check(deviation_range, &successful_boxes)? {
//         // Removing largest deviation outlier
//         successful_boxes = remove_largest_local_deviation_datapoint(&successful_boxes)?;

//         if (successful_boxes.len() as i64) < consensus_num {
//             return Err(CollectionError::FailedToReachConsensus().into());
//         }
//     }

//     // Return average + successful Datapoint boxes
//     Ok((average_datapoints(&successful_boxes)?, successful_boxes))
// }

// /// Verifies that the list of sorted Datapoint boxes passes the deviation check
// // pub fn deviation_check(
// //     deviation_range: i64,
//     datapoint_boxes: &Vec<ErgoBox>,
// ) -> Result<bool, CollectionError> {
//     let num = datapoint_boxes.len();
//     let max_datapoint =
//         unwrap_long(&datapoint_boxes[0].additional_registers.get_ordered_values()[2])?;
//     let min_datapoint = unwrap_long(
//         &datapoint_boxes[num - 1]
//             .additional_registers
//             .get_ordered_values()[2],
//     )?;
//     let deviation_delta = max_datapoint * deviation_range / 100;

//     Ok(min_datapoint >= max_datapoint - deviation_delta)
// }

// /// Function for averaging datapoints from a list of Datapoint boxes.
// // pub fn average_datapoints(boxes: &Vec<ErgoBox>) -> Result<u64, CollectionError> {
// //     let datapoints_sum = boxes.iter().fold(Ok(0), |acc: Result<i64>, b| {
//         Ok(acc? + unwrap_long(&b.additional_registers.get_ordered_values()[2])?)
//     })?;
//     if boxes.is_empty() {
//         return Err(CollectionError::LocalOracleFailedToPostDatapoint().into());
//     }
//     let average = datapoints_sum / boxes.len() as i64;
//     Ok(average as u64)
// }

// /// Verifies that the list of sorted Datapoint boxes passes the deviation check
// // pub fn deviation_check(
// //     deviation_range: i64,
//     datapoint_boxes: &Vec<ErgoBox>,
// ) -> Result<bool, CollectionError> {
//     let num = datapoint_boxes.len();
//     let max_datapoint =
//         unwrap_long(&datapoint_boxes[0].additional_registers.get_ordered_values()[2])?;
//     let min_datapoint = unwrap_long(
//         &datapoint_boxes[num - 1]
//             .additional_registers
//             .get_ordered_values()[2],
//     )?;
//     let deviation_delta = max_datapoint * deviation_range / 100;

//     Ok(min_datapoint >= max_datapoint - deviation_delta)
// }

// /// Removes boxes which do not have a valid datapoint Long in R6.
// // pub fn valid_boxes_filter(boxes: &Vec<ErgoBox>) -> Vec<ErgoBox> {
// //     let mut valid_boxes = vec![];
//     for b in boxes {
//         if unwrap_long(&b.additional_registers.get_ordered_values()[2]).is_ok() {
//             valid_boxes.push(b.clone());
//         }
//     }
//     valid_boxes
// }

// /// Finds whether the first or the last value in a list of sorted Datapoint boxes
// /// deviates more compared to their adjacted datapoint, and then removes
// /// said datapoint which deviates further.
// pub fn remove_largest_local_deviation_datapoint(
//     datapoint_boxes: &Vec<ErgoBox>,
// ) -> Result<Vec<ErgoBox>, CollectionError> {
//     let mut processed_boxes = datapoint_boxes.clone();

//     // Check if sufficient number of datapoint boxes to start removing
//     if datapoint_boxes.len() <= 2 {
//         Err(CollectionError::FailedToReachConsensus().into())
//     } else {
//         // Deserialize all the datapoints in a list
//         let dp_len = datapoint_boxes.len();
//         let datapoints: Vec<i64> = datapoint_boxes
//             .iter()
//             .map(|_| {
//                 unwrap_long(&datapoint_boxes[0].additional_registers.get_ordered_values()[2])
//                     .unwrap_or(0)
//             })
//             .collect();
//         // Check deviation by subtracting largest value by 2nd largest
//         let front_deviation = datapoints[0] - datapoints[1];
//         // Check deviation by subtracting 2nd smallest value by smallest
//         let back_deviation = datapoints[dp_len - 2] - datapoints[dp_len - 1];

//         // Remove largest datapoint if front deviation is greater
//         if front_deviation >= back_deviation {
//             processed_boxes.drain(0..1);
//         }
//         // Remove smallest datapoint if back deviation is greater
//         else {
//             processed_boxes.pop();
//         }
//         Ok(processed_boxes)
//     }
// }
