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
/// After every N connections (not logins) the server will
/// refresh failed and banned ip lists to remove old
/// entries that are no longer valid (if enough time has passed already).
const CONNECTIONS_TO_REFRESH_FAILED_AND_BANNED_LISTS: u64 = 30;

/// This struct represents an IP address of a
/// client who failed to login.
/// New failed attempts will cause the client's IP
/// to be banned.
#[derive(Debug)]
pub struct FailedIP {
    pub ip: IpAddr,
    pub failed_attempts_made: u32,
    pub last_attempt_time: DateTime<Local>,
}

/// This struct represents an IP address of a
/// client who failed to login multiple times.
#[derive(Debug)]
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
    /// Will be set to 0 every `CONNECTIONS_TO_REFRESH_FAILED_AND_BANNED_LISTS`
    /// connections.
    accepted_client_connections_to_refresh_lists: Arc<Mutex<u64>>,
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
            accepted_client_connections_to_refresh_lists: Arc::new(Mutex::new(0)),
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
            NetService::process_reporter_connections(
                listener_socker_reporters,
                logger_copy,
                connected_clone,
                database_clone,
            );
        });

        // Process clients.
        let logger_copy = self.logger.clone();
        let connected_clone = self.connected_socket_count.clone();
        let database_clone = self.database.clone();
        let failed_ips_clone = self.failed_ip_list.clone();
        let banned_ips_clone = self.banned_ip_list.clone();
        let accepted_connections_clone = self.accepted_client_connections_to_refresh_lists.clone();
        thread::spawn(move || {
            NetService::process_client_connections(
                listener_socker_clients,
                logger_copy,
                connected_clone,
                database_clone,
                failed_ips_clone,
                banned_ips_clone,
                accepted_connections_clone,
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
    fn process_reporter_connections(
        listener_socket: TcpListener,
        logger: Arc<Mutex<Logger>>,
        connected_count: Arc<Mutex<usize>>,
        database_manager: Arc<Mutex<DatabaseManager>>,
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
            let connected_count_clone = connected_count.clone();
            let database_clone = database_manager.clone();

            let handle = thread::Builder::new()
                .name(format!("reporter socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new_reporter(
                        logger_copy,
                        socket,
                        addr,
                        connected_count_clone,
                        database_clone,
                    );
                    user_service.process_reporter();
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
    fn process_client_connections(
        listener_socket: TcpListener,
        logger: Arc<Mutex<Logger>>,
        connected_count: Arc<Mutex<usize>>,
        database_manager: Arc<Mutex<DatabaseManager>>,
        failed_ip_list: Arc<Mutex<Vec<FailedIP>>>,
        banned_ip_list: Arc<Mutex<Vec<BannedIP>>>,
        accepted_client_connections_count: Arc<Mutex<u64>>,
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

            // Refresh failed and banned ip lists if needed.
            {
                let mut count_guard = accepted_client_connections_count.lock().unwrap();
                *count_guard += 1;
                if *count_guard == CONNECTIONS_TO_REFRESH_FAILED_AND_BANNED_LISTS {
                    *count_guard = 0;
                    NetService::refresh_failed_and_banned_lists(
                        &failed_ip_list,
                        &banned_ip_list,
                        &logger,
                    );
                }
            }

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
            } else {
                // Check if user failed to login before.
                let mut failed_list_guard = failed_ip_list.lock().unwrap();
                let failed_before = failed_list_guard.iter().find(|x| x.ip == addr.ip());

                if failed_before.is_some() {
                    // See if we can remove this ip from failed ips
                    // if the last failed attempt was too long ago.
                    let failed_before = failed_before.unwrap();
                    let time_diff = Local::now() - failed_before.last_attempt_time;

                    if time_diff.num_minutes() >= BAN_TIME_DURATION_IN_MIN {
                        let index_to_remove =
                            failed_list_guard.iter().position(|x| x.ip == addr.ip());
                        failed_list_guard.remove(index_to_remove.unwrap());
                    }
                }
            }

            let logger_copy = logger.clone();
            let banned_list_clone = banned_ip_list.clone();
            let failed_list_clone = failed_ip_list.clone();
            let connected_count_clone = connected_count.clone();
            let database_clone = database_manager.clone();

            let handle = thread::Builder::new()
                .name(format!("client socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new_client(
                        logger_copy,
                        socket,
                        addr,
                        connected_count_clone,
                        database_clone,
                        Some(failed_list_clone),
                        Some(banned_list_clone),
                    );
                    user_service.process_client();
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
    fn refresh_failed_and_banned_lists(
        failed_ip_list: &Arc<Mutex<Vec<FailedIP>>>,
        banned_ip_list: &Arc<Mutex<Vec<BannedIP>>>,
        logger: &Arc<Mutex<Logger>>,
    ) {
        // Refresh failed ips list.
        let mut _failed_list_len_before = 0;
        {
            let mut failed_list_guard = failed_ip_list.lock().unwrap();

            _failed_list_len_before = failed_list_guard.len();

            failed_list_guard.retain(|ip| {
                let time_diff = Local::now() - ip.last_attempt_time;
                time_diff.num_minutes() < BAN_TIME_DURATION_IN_MIN
            });
        }

        // Refresh banned ips list.
        let mut _banned_list_len_before = 0;
        {
            let mut banned_list_guard = banned_ip_list.lock().unwrap();

            _banned_list_len_before = banned_list_guard.len();

            banned_list_guard.retain(|ip| {
                let time_diff = Local::now() - ip.ban_start_time;
                time_diff.num_minutes() < BAN_TIME_DURATION_IN_MIN
            });
        }

        logger.lock().unwrap().print_and_log(&format!(
            "Refreshing failed and banned ip lists to remove old entries:\n\
            before:\n\
            - failed ip list size: {}\n\
            - banned ip list size: {}\n\
            after:\n\
            - failed ip list size: {}\n\
            - banned ip list size: {}.",
            _failed_list_len_before,
            _banned_list_len_before,
            failed_ip_list.lock().unwrap().len(),
            banned_ip_list.lock().unwrap().len()
        ));
    }
}
