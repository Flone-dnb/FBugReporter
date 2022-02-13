// External.
use serde::{Deserialize, Serialize};

// Custom.
use crate::misc::{GameReport, ReportResult};

// --------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub enum InReporterPacket {
    // should be exactly same as reporter's enum
    ReportPacket {
        reporter_net_protocol: u16,
        game_report: GameReport,
    },
}

#[derive(Serialize, Deserialize)]
pub enum OutReporterPacket {
    // should be exactly same as reporter's enum
    ReportAnswer { result_code: ReportResult },
}

// --------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub enum ClientLoginFailReason {
    // should be exactly same as client's enum
    WrongProtocol { server_protocol: u16 },
    WrongCredentials { ban_time_in_min: u64 },
    AlreadyConnected { ban_time_in_min: u64 },
}

#[derive(Serialize, Deserialize)]
pub enum InClientPacket {
    // should be exactly same as client's enum
    ClientLogin {
        client_net_protocol: u16,
        username: String,
        password: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize)]
pub enum OutClientPacket {
    // should be exactly same as client's enum
    ClientLoginAnswer {
        is_ok: bool,
        reason: Option<ClientLoginFailReason>,
    },
}

// --------------------------------------------------------
