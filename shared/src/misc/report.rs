// External.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: u64,
    pub title: String,
    pub game: String,
    pub date: String,
    pub time: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameReport {
    pub report_name: String,
    pub report_text: String,
    pub sender_name: String,
    pub sender_email: String,
    pub game_name: String,
    pub game_version: String,
    pub client_os_info: os_info::Info,
    // if adding new stuff here
    // also add its limit to the ReportLimits enum (in reporter and server)
    // and update the NETWORK_PROTOCOL_VERSION
    // and maybe update table structure in the database (backwards compatibility)?
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReportAttachment {
    pub file_name: String,
    pub data: Vec<u8>,
}

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
    /// Returns the maximum amount of __characters__ allowed for the field.
    pub fn max_length(&self) -> usize {
        match *self {
            ReportLimits::ReportName => 50,   // the server also checks the limits
            ReportLimits::ReportText => 5120, // so if changing any values here
            ReportLimits::SenderName => 50,   // also change them in the server
            ReportLimits::SenderEMail => 50,
            ReportLimits::GameName => 50,
            ReportLimits::GameVersion => 50,
            // if adding new fields, update is_input_valid() in lib.rs (in reporter)
            // also update get_field_limit()
            // also update/add get_field_limit() calls in 'example'
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
            // add new fields here, continue 6, 7... (don't add stuff before existing values)
        }
    }
}

/// Values that reporter returns into game engine.
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ReportResult {
    Ok,
    ServerNotSet,
    InvalidInput,
    CouldNotConnect,
    InternalError,
    WrongProtocol,
    ServerRejected,
    NetworkIssue,
    AttachmentDoesNotExist,
    AttachmentTooBig,
    // make sure to handle new entries
    // in the 'example' project
}
impl ReportResult {
    pub fn value(&self) -> i32 {
        match *self {
            ReportResult::Ok => 0,
            ReportResult::ServerNotSet => 1,
            ReportResult::InvalidInput => 2,
            ReportResult::CouldNotConnect => 3,
            ReportResult::InternalError => 4,
            ReportResult::WrongProtocol => 5,
            ReportResult::ServerRejected => 6,
            ReportResult::NetworkIssue => 7,
            ReportResult::AttachmentDoesNotExist => 8,
            ReportResult::AttachmentTooBig => 9,
        }
    }
}
