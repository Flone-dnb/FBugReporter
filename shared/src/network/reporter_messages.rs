// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::report::*;

/// Reporter's request to the server.
/// If made changes, change protocol version.
#[derive(Serialize, Deserialize, Debug)]
pub enum ReporterRequest {
    Report {
        reporter_net_protocol: u16,
        game_report: Box<GameReport>,
        attachments: Vec<ReportAttachment>,
    },
    /// Max attachment size (in total) in MB.
    MaxAttachmentSize {},
}

/// Server's answer to reporter.
/// If made changes, change protocol version.
#[derive(Serialize, Deserialize, Debug)]
pub enum ReporterAnswer {
    Report { result_code: ReportResult },
    MaxAttachmentSize { max_attachments_size_in_mb: usize },
}
