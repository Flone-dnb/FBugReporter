// External.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum OutPacket {
    // should be in sync with server's enum
    ReportPacket {}, // not used in client
    ClientLogin {
        client_net_protocol: u16,
        username: String,
        password: Vec<u8>,
    },
}
