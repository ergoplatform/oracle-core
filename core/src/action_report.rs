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

#[derive(Debug)]
pub struct ActionReportStorage {
    refresh: Option<RefreshActionReport>,
    publish_datapoint: Option<PublishDatapointActionReport>,
}

impl ActionReportStorage {
    pub fn new() -> Self {
        Self {
            refresh: None,
            publish_datapoint: None,
        }
    }

    pub fn add(&mut self, report: PoolActionReport) {
        match report {
            PoolActionReport::Refresh(report) => self.refresh = Some(report),
            PoolActionReport::PublishDatapoint(report) => self.publish_datapoint = Some(report),
        }
    }

    pub fn get_last_refresh_report(&self) -> Option<&RefreshActionReport> {
        self.refresh.as_ref()
    }
}
