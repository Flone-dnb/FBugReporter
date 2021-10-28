use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GameReport {
    pub report_name: String,
    pub report_text: String,
    pub sender_name: String,
    pub sender_email: String,
    pub game_name: String,
    pub game_version: String,
    pub client_os_info: os_info::Info,
}
