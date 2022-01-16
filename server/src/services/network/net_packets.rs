// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::{GameReport, ReportResult};

#[derive(Serialize, Deserialize)]
pub enum InPacket {
    // should be in sync with reporter's/client's enum
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
    },
}

#[derive(Serialize, Deserialize)]
pub enum OutPacket {
    // should be in sync with reporter's/client's enum
    ReportAnswer { result_code: ReportResult },
}
