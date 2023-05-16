use crate::action_report::RefreshActionReport;

use super::RefreshAction;

pub struct RefreshActionResult {
    action: RefreshAction,
    report: RefreshActionReport,
}
