// External.
use serde::{Deserialize, Serialize};

#[derive(Debug)]
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
            // be careful and tweak the MAX_PACKET_SIZE_IN_BYTES in user_service.rs
            // if needed, always test first, only then change MAX_PACKET_SIZE_IN_BYTES
            ReportLimits::ReportName => 50, // the reporter also checks the limits
            ReportLimits::ReportText => 5120, // so if changing any values here
            ReportLimits::SenderName => 50, // also change them in the reporter
            ReportLimits::SenderEMail => 50,
            ReportLimits::GameName => 50,
            ReportLimits::GameVersion => 50,
            // if adding new fields, update check_report_field_limits() in user_service.rs
        }
    }
}
// --------------------------------------------------------
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ReportResult {
    // this enum should be in sync with the server's enum
    Ok,
    ServerNotSet,
    InvalidInput,
    CouldNotConnect,
    InternalError,
    WrongProtocol,
    ServerRejected,
    NetworkIssue,
    // this enum should be in sync with the server's enum
}
// impl ReportResult {
//     pub fn value(&self) -> i32 {
//         match *self {
//             ReportResult::Ok => 0,
//             ReportResult::ServerNotSet => 1, // if changing any values here
//             ReportResult::InvalidInput => 2, // also change them in the reporter
//             ReportResult::CouldNotConnect => 3, // ...
//             ReportResult::InternalError => 4,
//             ReportResult::WrongProtocol => 5,
//             ReportResult::ServerRejected => 6,
//             ReportResult::NetworkIssue => 7,
//         }
//     }
// }
// --------------------------------------------------------
