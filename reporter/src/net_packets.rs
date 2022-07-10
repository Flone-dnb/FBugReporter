// External.
use serde::{Deserialize, Serialize};

// Custom.
use shared::report::*;

// should be exactly the same as server's struct
#[derive(Serialize, Deserialize)]
pub enum OutPacket {
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
        attachments: Vec<ReportAttachment>,
    },
}

// should be exactly the same as server's struct
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum InPacket {
    ReportAnswer { result_code: ReportResult },
}
