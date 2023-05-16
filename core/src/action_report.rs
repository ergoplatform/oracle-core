use derive_more::From;
use ergo_lib::ergo_chain_types::EcPoint;

use crate::oracle_types::Rate;

#[derive(Debug)]
pub struct RefreshActionReport {
    pub oracle_boxes_collected: Vec<EcPoint>,
}

#[derive(Debug)]
pub struct PublishDatapointActionReport {
    pub posted_datapoint: Rate,
}

#[derive(Debug, From)]
pub enum PoolActionReport {
    Refresh(RefreshActionReport),
    PublishDatapoint(PublishDatapointActionReport),
}
