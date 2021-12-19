use serde::{Deserialize, Serialize};

// --------------------------------------------------------
pub enum ReportLimits {
    ReportName,
    ReportText,
    SenderName,
    SenderEMail,
    GameName,
    GameVersion,
}
impl ReportLimits {
    /// Returns the maximum number of __characters__ allowed for the field.
    pub fn max_length(&self) -> usize {
        match *self {
            ReportLimits::ReportName => 50,
            ReportLimits::ReportText => 5120,
            ReportLimits::SenderName => 50,
            ReportLimits::SenderEMail => 50,
            ReportLimits::GameName => 50,
            ReportLimits::GameVersion => 50,
        }
    }
    pub fn id(&self) -> u64 {
        match *self {
            ReportLimits::ReportName => 0,
            ReportLimits::ReportText => 1,
            ReportLimits::SenderName => 2,
            ReportLimits::SenderEMail => 3,
            ReportLimits::GameName => 4,
            ReportLimits::GameVersion => 5,
        }
    }
}
// --------------------------------------------------------
// --------------------------------------------------------
#[derive(PartialEq)]
pub enum ReportResult {
    Ok,
    ServerNotSet,
    InvalidInput,
    CouldNotConnect,
    InternalError,
}
impl ReportResult {
    pub fn value(&self) -> i32 {
        match *self {
            ReportResult::Ok => 0,
            ReportResult::ServerNotSet => 1,
            ReportResult::InvalidInput => 2,
            ReportResult::CouldNotConnect => 3,
            ReportResult::InternalError => 4,
        }
    }
}
// --------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct GameReport {
    pub report_name: String,
    pub report_text: String,
    pub sender_name: String,
    pub sender_email: String,
    pub game_name: String,
    pub game_version: String,
    pub client_os_info: os_info::Info,
    // if adding new stuff here
    // also add its limit to the ReportLimits enum
}
