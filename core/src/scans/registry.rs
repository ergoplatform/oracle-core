use crate::node_interface::node_api::NodeApi;
use crate::node_interface::node_api::NodeApiError;

use super::NodeScan;

pub struct NodeScanRegistry<'a> {
    // TODO: own scans
    pub scans: Vec<&'a dyn NodeScan>,
}

impl<'a> NodeScanRegistry<'a> {
    pub fn new() -> NodeScanRegistry<'static> {
        NodeScanRegistry { scans: Vec::new() }
    }

    pub fn register(&mut self, scan: &'a dyn NodeScan) {
        self.scans.push(scan);
    }

    pub fn deregister_all_scans(self, node_api: &NodeApi) -> Result<(), NodeApiError> {
        for scan in self.scans {
            node_api.deregister_scan(scan.scan_id())?;
        }
        Ok(())
    }
}
