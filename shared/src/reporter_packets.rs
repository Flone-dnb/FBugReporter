// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::report::*;

/// Reporter's request to the server.
/// If made changes, change protocol version.
#[derive(Serialize, Deserialize)]
pub enum ReporterRequest {
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
        attachments: Vec<ReportAttachment>,
    },
}

/// Server's answer to reporter.
/// If made changes, change protocol version.
#[derive(Serialize, Deserialize)]
pub enum ReporterAnswer {
    ReportRequestResult { result_code: ReportResult },
}
