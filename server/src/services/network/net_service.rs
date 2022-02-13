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

// External.
use chrono::{DateTime, Local};
use sha2::{Digest, Sha512};

const WRONG_PASSWORD_FIRST_BAN_TIME_DURATION_IN_MIN: u64 = 5;
// if a client used wrong password for the second time,
// new ban duration will be
// 'WRONG_PASSWORD_FIRST_BAN_TIME_DURATION_IN_MIN' multiplied by this value.
const FAILED_ATTEMPT_BAN_TIME_DURATION_MULTIPLIER: u64 = 2;

struct BannedIP {
    ip: IpAddr,
    ban_start_time: DateTime<Local>,
    current_ban_duration_in_min: u64,
}

pub struct NetService {
    pub logger: Arc<Mutex<Logger>>,
    pub server_config: ServerConfig,
    connected_socket_count: Arc<Mutex<usize>>,
    database: Arc<Mutex<DatabaseManager>>,
    banned_ip_list: Vec<BannedIP>,
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
            banned_ip_list: Vec::new(),
        })
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
    /// Adds a new user to the database.
    ///
    /// On success returns user's password.
    /// On failure returns error description via `AppError`.
    pub fn add_user(&mut self, username: &str) -> Result<String, AppError> {
        let result = self.database.lock().unwrap().add_user(username);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        } else {
            return Ok(result.unwrap());
        }
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

        self.process_users(listener_socket);
    }
    fn process_users(&mut self, listener_socket: TcpListener) {
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
