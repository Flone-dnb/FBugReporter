use super::config_service::ServerConfig;

const SERVER_PROTOCOL_VERSION: u16 = 0;
pub const SERVER_DEFAULT_PORT: u16 = 61919;

pub struct NetService {
    pub server_config: ServerConfig,
}

impl NetService {
    pub fn new() -> Result<Self, String> {
        let config = ServerConfig::new();
        if let Err(e) = config {
            return Err(format!("{} at [{}, {}]\n\n", e, file!(), line!()));
        }

        Ok(Self {
            server_config: config.unwrap(),
        })
    }
}
