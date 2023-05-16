use ergo_lib::ergo_chain_types::EcPoint;

#[derive(Debug)]
pub struct RefreshActionReport {
    pub oracle_boxes_collected: Vec<EcPoint>,
}
