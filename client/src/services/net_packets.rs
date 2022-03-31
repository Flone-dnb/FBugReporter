// External.
use serde::{Deserialize, Serialize};

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
    // should be exactly the same as server's enum
    WrongProtocol { server_protocol: u16 },
    WrongCredentials { result: ClientLoginFailResult },
    NeedFirstPassword, // user just registered, the server is waiting for a new password to set
    NeedOTP,
    SetupOTP { qr_code: String },
}

#[derive(Serialize, Deserialize)]
pub enum OutClientPacket {
    // should be exactly the same as server's enum
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
}

// should be exactly the same as server's struct
#[derive(Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: u64,
    pub title: String,
    pub game: String,
    pub date: String,
    pub time: String,
}

#[derive(Serialize, Deserialize)]
pub enum InClientPacket {
    // should be exactly the same as server's enum
    LoginAnswer {
        is_ok: bool,
        fail_reason: Option<ClientLoginFailReason>,
    },
    ReportsSummary {
        reports: Vec<ReportSummary>,
    },
}
