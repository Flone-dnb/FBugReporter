// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::*;

#[derive(Serialize, Deserialize)]
pub enum OutPacket {
    // should be in sync with server's enum
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum InPacket {
    // should be in sync with server's enum
    ReportAnswer { result_code: ReportResult },
}
