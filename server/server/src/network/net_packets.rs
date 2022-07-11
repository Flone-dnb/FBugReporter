// External.
use serde::{Deserialize, Serialize};

// Custom.
use shared::report::*;

// --------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub enum InReporterPacket {
    // should be exactly same as reporter's enum
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
        attachments: Vec<ReportAttachment>,
    },
}

#[derive(Serialize, Deserialize)]
pub enum OutReporterPacket {
    // should be exactly same as reporter's enum
    ReportAnswer { result_code: ReportResult },
}

// --------------------------------------------------------

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

#[derive(Serialize, Deserialize)]
pub enum ClientLoginFailReason {
    // should be exactly same as client's enum
    WrongProtocol { server_protocol: u16 },
    WrongCredentials { result: ClientLoginFailResult },
    NeedFirstPassword, // user just registered, we are waiting for a new password to set
    NeedOTP,
    SetupOTP { qr_code: String },
}

#[derive(Serialize, Deserialize)]
pub enum InClientPacket {
    // should be exactly same as client's enum
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
    DeleteReport {
        report_id: u64,
    },
}

#[derive(Serialize, Deserialize)]
pub enum OutClientPacket {
    // should be exactly same as client's enum
    LoginAnswer {
        is_ok: bool,
        is_admin: bool,
        fail_reason: Option<ClientLoginFailReason>,
    },
    ReportsSummary {
        reports: Vec<ReportSummary>,
        total_reports: u64,
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
    },
    DeleteReportResult {
        is_found_and_removed: bool,
    },
}

// --------------------------------------------------------
