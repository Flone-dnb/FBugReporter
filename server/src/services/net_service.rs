use super::config_service::ServerConfig;

const SERVER_PROTOCOL_VERSION: u16 = 0;
pub const SERVER_PASSWORD_BIT_COUNT: u64 = 1024;

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
    pub fn refresh_password(&mut self) -> Result<(), String> {
        if let Err(msg) = self.server_config.refresh_password() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn refresh_port(&mut self) -> Result<(), String> {
        if let Err(msg) = self.server_config.refresh_port() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn start(&self) -> Result<(), String> {
        Ok(())
    }
}
