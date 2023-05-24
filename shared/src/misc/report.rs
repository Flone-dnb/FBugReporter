// Std.
use std::str::FromStr;

// External.
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: u64,
    pub title: String,
    pub game: String,
    pub date: String,
    pub time: String,
}

/// Represents a report that the reporter sends.
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

/// Represents a report that we store in the database and send
/// to clients.
pub struct ReportData {
    pub id: u64,
    pub title: String,
    pub game_name: String,
    pub game_version: String,
    pub text: String,
    pub date: String,
    pub time: String,
    pub sender_name: String,
    pub sender_email: String,
    pub os_info: String,
    pub attachments: Vec<ReportAttachmentSummary>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReportAttachmentSummary {
    pub id: usize,
    pub file_name: String,
    pub size_in_bytes: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReportAttachment {
    pub file_name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, EnumString, Display)]
pub enum ReportLimits {
    ReportName,
    ReportText,
    SenderName,
    SenderEmail,
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
            ReportLimits::SenderEmail => 50,
            ReportLimits::GameName => 50,
            ReportLimits::GameVersion => 50,
            // if adding new fields, update is_input_valid() in lib.rs (in reporter)
            // also update get_field_limit()
            // also update/add get_field_limit() calls in 'example'
        }
    }
    pub fn from_string(name: &str) -> Option<ReportLimits> {
        let result = ReportLimits::from_str(name);
        if let Err(_) = result {
            return None;
        }

        return Some(result.unwrap());
    }
}

/// Values that the reporter returns into the game engine.
#[derive(PartialEq, Debug, Clone)]
pub enum ReportResult {
    Ok,
    ServerNotSet,
    InvalidInput,
    CouldNotConnect,
    AttachmentDoesNotExist,
    AttachmentTooBig,
    Other(String),
    // make sure to handle new entries in the 'example' project
}

impl ReportResult {
    pub fn value(&self) -> i32 {
        match &self {
            ReportResult::Ok => 0,
            ReportResult::ServerNotSet => 1,
            ReportResult::InvalidInput => 2,
            ReportResult::CouldNotConnect => 3,
            ReportResult::AttachmentDoesNotExist => 4,
            ReportResult::AttachmentTooBig => 5,
            ReportResult::Other(_) => 6,
        }
    }
}
