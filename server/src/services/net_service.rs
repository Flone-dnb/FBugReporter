// Std.
use std::io::prelude::*;
use std::net::*;

// Custom.
use super::config_service::ServerConfig;
use super::logger_service::Logger;

const SERVER_PROTOCOL_VERSION: u16 = 0;
pub const SERVER_PASSWORD_BIT_COUNT: u64 = 1024;

pub struct NetService {
    pub logger: Logger,
    pub server_config: ServerConfig,
}

impl NetService {
    pub fn new(logger: Logger) -> Result<Self, String> {
        let config = ServerConfig::new();
        if let Err(e) = config {
            return Err(format!("{} at [{}, {}]\n\n", e, file!(), line!()));
        }

        Ok(Self {
            server_config: config.unwrap(),
            logger,
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
    pub fn set_port(&mut self, port: u16) -> Result<(), String> {
        if let Err(msg) = self.server_config.set_port(port) {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn start(&self) -> Result<(), String> {
        // Create socket.
        let listener_socker =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.server_port));
        if let Err(e) = listener_socker {
            let error = format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            );
            self.logger.log(&error);
            return Err(error);
        }
        let listener_socket = listener_socker.unwrap();

        // Wait for connection.
        let accept_result = listener_socket.accept();
        if let Err(e) = accept_result {
            let error = format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            );
            self.logger.log(&error);
            return Err(error);
        }

        let (mut socket, addr) = accept_result.unwrap();
        self.logger
            .log(&format!("Accepted connection from port {}.", addr.port()));

        if let Err(e) = socket.set_nodelay(true) {
            let error = format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            );
            self.logger.log(&error);
            return Err(error);
        }

        // TODO: 1. establish secure connection
        // TODO: in wouldblock have loop limit as a variable
        // never set the limit when waiting for user messages!
        // TODO: 2. check password hash and etc

        Ok(())
    }
}
