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

/// If a user failed to login more than this value times,
/// he will be banned. This value can be 0 to ban
/// users on first failed login attempt.
pub const MAX_ALLOWED_FAILED_LOGIN_ATTEMPTS_UNTILL_BAN: u32 = 3;
/// The amount of time a banned user will have to wait
/// until he can try to login again.
pub const BAN_TIME_DURATION_IN_MIN: i64 = 5;

/// This struct represents an IP address of a
/// client who failed to login.
/// New failed attempts will cause the client's IP
/// to be banned.
pub struct FailedIP {
    pub ip: IpAddr,
    pub failed_attempts_made: u32,
}

/// This struct represents an IP address of a
/// client who failed to login multiple times.
pub struct BannedIP {
    pub ip: IpAddr,
    pub ban_start_time: DateTime<Local>,
    pub current_ban_duration_in_min: i64,
}

pub struct NetService {
    pub logger: Arc<Mutex<Logger>>,
    pub server_config: ServerConfig,
    connected_socket_count: Arc<Mutex<usize>>,
    database: Arc<Mutex<DatabaseManager>>,
    failed_ip_list: Arc<Mutex<Vec<FailedIP>>>,
    banned_ip_list: Arc<Mutex<Vec<BannedIP>>>,
}

impl NetService {
    /// Creates a new instance of the `NetService`.
    ///
    /// Returns `AppError` if something went wrong
    /// when initializing/connecting to the database.
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
            failed_ip_list: Arc::new(Mutex::new(Vec::new())),
            banned_ip_list: Arc::new(Mutex::new(Vec::new())),
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
        let failed_ips_clone = self.failed_ip_list.clone();
        let banned_ips_clone = self.banned_ip_list.clone();
        thread::spawn(move || {
            NetService::process_connection(
                listener_socker_reporters,
                logger_copy,
                connected_clone,
                database_clone,
                failed_ips_clone,
                banned_ips_clone,
                true,
            );
        });

        // Process clients.
        let logger_copy = self.logger.clone();
        let connected_clone = self.connected_socket_count.clone();
        let database_clone = self.database.clone();
        let failed_ips_clone = self.failed_ip_list.clone();
        let banned_ips_clone = self.banned_ip_list.clone();
        thread::spawn(move || {
            NetService::process_connection(
                listener_socker_clients,
                logger_copy,
                connected_clone,
                database_clone,
                failed_ips_clone,
                banned_ips_clone,
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
        failed_ip_list: Arc<Mutex<Vec<FailedIP>>>,
        banned_ip_list: Arc<Mutex<Vec<BannedIP>>>,
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

            if is_reporter == false {
                // Check if this IP is banned.
                let mut banned_list_guard = banned_ip_list.lock().unwrap();
                let is_banned = banned_list_guard.iter().find(|x| x.ip == addr.ip());

                if is_banned.is_some() {
                    // This IP is banned, see if ban time is over.
                    let banned_ip = is_banned.unwrap();
                    let time_diff = Local::now() - banned_ip.ban_start_time;

                    if time_diff.num_minutes() < banned_ip.current_ban_duration_in_min {
                        logger.lock().unwrap().print_and_log(&format!(
                            "Banned IP address ({}) attempted to connect. \
                            Connection was rejected.",
                            banned_ip.ip.to_string()
                        ));
                        continue; // still banned
                    } else {
                        // Remove from banned ips.
                        let index_to_remove = banned_list_guard
                            .iter()
                            .position(|x| x.ip == addr.ip())
                            .unwrap();
                        banned_list_guard.remove(index_to_remove);
                    }
                }
            }

            let logger_copy = logger.clone();
            let banned_list_clone = banned_ip_list.clone();
            let failed_list_clone = failed_ip_list.clone();
            let connected_count_clone = connected_count.clone();
            let database_clone = database_manager.clone();

            let handle = thread::Builder::new()
                .name(format!("socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new(
                        logger_copy,
                        socket,
                        addr,
                        connected_count_clone,
                        database_clone,
                        failed_list_clone,
                        banned_list_clone,
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
