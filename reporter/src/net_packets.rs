// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::GameReport;

#[derive(Serialize, Deserialize)]
pub struct ReportPacket {
    pub reporter_net_protocol: u16,
    pub game_report: GameReport,
}
