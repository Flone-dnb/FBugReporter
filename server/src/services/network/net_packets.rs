// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::GameReport;

#[derive(Serialize, Deserialize)]
pub enum NetPacket {
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
    },
}
