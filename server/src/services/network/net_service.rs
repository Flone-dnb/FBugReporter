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

const WRONG_PASSWORD_FIRST_BAN_TIME_DURATION_IN_MIN: u64 = 1;
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

        let db = DatabaseManager::new();
        if let Err(err) = db {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(Self {
            server_config: config,
            logger: Arc::new(Mutex::new(logger)),
            connected_socket_count: Arc::new(Mutex::new(0)),
            database: Arc::new(Mutex::new(db.unwrap())),
            banned_ip_list: Vec::new(),
        })
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

        // Create socket for reporters.
        let listener_socker_reporters =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.port_for_reporters));
        if let Err(ref e) = listener_socker_reporters {
            self.logger.lock().unwrap().print_and_log(&format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let listener_socker_reporters = listener_socker_reporters.unwrap();

        // Create socket for clients.
        let listener_socker_clients =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.port_for_clients));
        if let Err(ref e) = listener_socker_clients {
            self.logger.lock().unwrap().print_and_log(&format!(
                "An error occurred at [{}, {}]: {:?}\n\n",
                file!(),
                line!(),
                e
            ));
        }
        let listener_socker_clients = listener_socker_clients.unwrap();

        // Process reporters.
        let logger_copy = self.logger.clone();
        let connected_clone = self.connected_socket_count.clone();
        let database_clone = self.database.clone();
        thread::spawn(move || {
            NetService::process_connection(
                listener_socker_reporters,
                logger_copy,
                connected_clone,
                database_clone,
                true,
            );
        });

        // Process clients.
        let logger_copy = self.logger.clone();
        let connected_clone = self.connected_socket_count.clone();
        let database_clone = self.database.clone();
        thread::spawn(move || {
            NetService::process_connection(
                listener_socker_clients,
                logger_copy,
                connected_clone,
                database_clone,
                false,
            );
        });

        let logger_guard = self.logger.lock().unwrap();
        logger_guard.print_and_log(&format!(
            "Ready to accept client connections on port {}",
            self.server_config.port_for_clients
        ));
        logger_guard.print_and_log(&format!(
            "Ready to accept reporter connections on port {}",
            self.server_config.port_for_reporters
        ));
    }
    fn process_connection(
        listener_socket: TcpListener,
        logger: Arc<Mutex<Logger>>,
        connected_count: Arc<Mutex<usize>>,
        database_manager: Arc<Mutex<DatabaseManager>>,
        is_reporter: bool,
    ) {
        loop {
            // Wait for connection.
            let accept_result = listener_socket.accept();
            if let Err(ref e) = accept_result {
                logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }

            let (socket, addr) = accept_result.unwrap();

            if let Err(e) = socket.set_nodelay(true) {
                logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }
            if let Err(e) = socket.set_nonblocking(true) {
                logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }

            let logger_copy = logger.clone();
            let connected_clone = connected_count.clone();
            let database_clone = database_manager.clone();

            let handle = thread::Builder::new()
                .name(format!("socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new(
                        logger_copy,
                        socket,
                        addr,
                        connected_clone,
                        database_clone,
                        is_reporter,
                    );
                    if is_reporter {
                        user_service.process_reporter();
                    } else {
                        user_service.process_client();
                    }
                });
            if let Err(ref e) = handle {
                logger.lock().unwrap().print_and_log(&format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ));
            }
        }
    }
}
