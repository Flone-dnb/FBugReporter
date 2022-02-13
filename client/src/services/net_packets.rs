// External.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ClientLoginFailReason {
    // should be exactly the same as server's enum
    WrongProtocol { server_protocol: u16 },
    WrongCredentials { ban_time_in_min: u64 },
    AlreadyConnected { ban_time_in_min: u64 },
}

#[derive(Serialize, Deserialize)]
pub enum OutClientPacket {
    // should be exactly the same as server's enum
    ClientLogin {
        client_net_protocol: u16,
        username: String,
        password: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize)]
pub enum InClientPacket {
    // should be exactly the same as server's enum
    ClientLoginAnswer {
        is_ok: bool,
        reason: Option<ClientLoginFailReason>,
    },
}
