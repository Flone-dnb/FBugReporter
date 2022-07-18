// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::report::{ReportAttachmentSummary, ReportSummary};

/// Client's request to the server.
/// If made changes, change protocol version.
#[derive(Serialize, Deserialize)]
pub enum ClientRequest {
    Login {
        client_net_protocol: u16,
        username: String,
        password: Vec<u8>,
        otp: String,
    },
    SetFirstPassword {
        client_net_protocol: u16,
        username: String,
        old_password: Vec<u8>,
        new_password: Vec<u8>,
    },
    QueryReportsSummary {
        page: u64,
        amount: u64,
    },
    QueryReport {
        report_id: u64,
    },
    QueryAttachment {
        attachment_id: usize,
    },
    DeleteReport {
        report_id: u64,
    },
}

/// Server's answer to the client.
/// If made changes, change protocol version.
#[derive(Serialize, Deserialize)]
pub enum ClientAnswer {
    LoginAnswer {
        is_ok: bool,
        is_admin: bool,
        fail_reason: Option<ClientLoginFailReason>,
    },
    ReportsSummary {
        reports: Vec<ReportSummary>,
        total_reports: u64,
        total_disk_space_mb: u64,
        used_disk_space_mb: u64,
    },
    Report {
        id: u64,
        title: String,
        game_name: String,
        game_version: String,
        text: String,
        date: String,
        time: String,
        sender_name: String,
        sender_email: String,
        os_info: String,
        attachments: Vec<ReportAttachmentSummary>,
    },
    Attachment {
        is_found: bool,
        data: Vec<u8>,
    },
    DeleteReportResult {
        is_found_and_removed: bool,
    },
}

/// If made changes, change protocol version.
#[derive(Serialize, Deserialize)]
pub enum ClientLoginFailResult {
    FailedAttempt {
        failed_attempts_made: u32,
        max_failed_attempts: u32,
    },
    Banned {
        ban_time_in_min: i64,
    },
}

/// If made changes, change protocol version.
#[derive(Serialize, Deserialize)]
pub enum ClientLoginFailReason {
    WrongProtocol { server_protocol: u16 },
    WrongCredentials { result: ClientLoginFailResult },
    NeedFirstPassword, // user just registered, we are waiting for a new password to set
    NeedOTP,
    SetupOTP { qr_code: String },
}
