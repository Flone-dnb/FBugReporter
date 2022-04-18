// External.
use serde::{Deserialize, Serialize};

// should be exactly the same as client's struct
#[derive(Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: u64,
    pub title: String,
    pub game: String,
    pub date: String,
    pub time: String,
}

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
    // and update the NETWORK_PROTOCOL_VERSION
    // and maybe update table structure in the database (backwards compatibility)?
}
