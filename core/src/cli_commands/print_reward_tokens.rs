use crate::{
    box_kind::OracleBox,
    oracle_state::{LocalDatapointBoxSource, StageError},
};

pub fn print_reward_tokens(
    local_datapoint_box_source: &dyn LocalDatapointBoxSource,
) -> Result<(), StageError> {
    if let Some(oracle_box) = local_datapoint_box_source.get_local_oracle_datapoint_box()? {
        let num_tokens = *oracle_box.reward_token().amount.as_u64();
        if num_tokens == 0 {
            println!("Oracle box contains zero reward tokens");
        } else {
            println!("Number of claimable reward tokens: {}", num_tokens - 1);
        }
    } else {
        println!("No datapoint box exists");
    }
    Ok(())
}
