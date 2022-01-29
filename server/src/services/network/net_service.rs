// Std.
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;

// Custom.
use crate::error::AppError;
use crate::services::config_service::ServerConfig;
use crate::services::db_manager::DatabaseManager;
use crate::services::logger_service::Logger;
use crate::services::network::user_service::UserService;

pub struct NetService {
    pub logger: Arc<Mutex<Logger>>,
    pub server_config: ServerConfig,
    connected_socket_count: Arc<Mutex<usize>>,
    database: Arc<Mutex<DatabaseManager>>,
}

impl NetService {
    pub fn new(logger: Logger) -> Result<Self, AppError> {
        let config = ServerConfig::new();
        if let Err(err) = config {
            return Err(err.add_entry(file!(), line!()));
        }

        let db = DatabaseManager::new();
        if let Err(err) = config {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(Self {
            server_config: config.unwrap(),
            logger: Arc::new(Mutex::new(logger)),
            connected_socket_count: Arc::new(Mutex::new(0)),
            database: Arc::new(Mutex::new(db.unwrap())),
        })
    }
    pub fn refresh_password(&mut self) -> Result<(), AppError> {
        if let Err(err) = self.server_config.refresh_password() {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(())
    }
    pub fn refresh_port(&mut self) -> Result<(), AppError> {
        if let Err(err) = self.server_config.refresh_port() {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(())
    }
    pub fn set_port(&mut self, port: u16) -> Result<(), AppError> {
        if let Err(err) = self.server_config.set_port(port) {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(())
    }
    pub fn start(&mut self) {
        {
            self.logger.lock().unwrap().print_and_log("Starting...");
        }

        // Create socket.
        let listener_socker =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.server_port));
        if let Err(ref e) = listener_socker {
            self.logger.lock().unwrap().print_and_log(&format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let listener_socket = listener_socker.unwrap();

        {
            self.logger.lock().unwrap().print_and_log(&format!(
                "Ready to accept connections on port {}",
                self.server_config.server_port
            ));
        }

        loop {
            // Wait for connection.
            let accept_result = listener_socket.accept();
            if let Err(ref e) = accept_result {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }

            let (socket, addr) = accept_result.unwrap();

            if let Err(e) = socket.set_nodelay(true) {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }
            if let Err(e) = socket.set_nonblocking(true) {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }

            let logger_copy = self.logger.clone();
            let connected_clone = self.connected_socket_count.clone();
            let database_clone = self.database.clone();

            let handle = thread::Builder::new()
                .name(format!("socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new(
                        logger_copy,
                        socket,
                        addr,
                        connected_clone,
                        database_clone,
                    );
                    user_service.process_user();
                });
            if let Err(ref e) = handle {
                self.logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }
        }
    }
}
