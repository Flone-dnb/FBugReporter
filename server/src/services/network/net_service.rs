// Std.
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;

// Custom.
use crate::error::AppError;
use crate::services::{
    config_service::ServerConfig,
    db_manager::{AddUserResult, DatabaseManager},
    logger_service::*,
    network::{ban_manager::BanManager, user_service::UserService},
};

pub struct NetService {
    pub logger: Arc<Mutex<Logger>>,
    pub server_config: Arc<ServerConfig>,
    connected_socket_count: Arc<Mutex<usize>>,
    database: Arc<Mutex<DatabaseManager>>,
    ban_manager: Arc<Mutex<BanManager>>,
}

impl NetService {
    /// Creates a new instance of the `NetService`.
    ///
    /// Returns `AppError` if something went wrong
    /// when initializing/connecting to the database.
    pub fn new(logger: Logger) -> Result<Self, AppError> {
        let config = Arc::new(ServerConfig::new());

        let db = DatabaseManager::new();
        if let Err(err) = db {
            return Err(err.add_entry(file!(), line!()));
        }

        let logger = Arc::new(Mutex::new(logger));

        Ok(Self {
            server_config: config.clone(),
            logger: logger.clone(),
            connected_socket_count: Arc::new(Mutex::new(0)),
            database: Arc::new(Mutex::new(db.unwrap())),
            ban_manager: Arc::new(Mutex::new(BanManager::new(logger, config))),
        })
    }
    /// Adds a new user to the database.
    ///
    /// Parameters:
    /// - `username` login of the new user
    /// - `is_admin` whether the user should have admin privileges or not
    /// (be able to delete reports using the client application).
    pub fn add_user(&mut self, username: &str, is_admin: bool) -> AddUserResult {
        let result = self.database.lock().unwrap().add_user(username, is_admin);
        if let AddUserResult::Error(e) = result {
            return AddUserResult::Error(e.add_entry(file!(), line!()));
        } else {
            return result;
        }
    }
    /// Removes the user from the database.
    ///
    /// Returns `Ok(true)` if the user was found and removed,
    /// `Ok(false)` if the user was not found.
    /// On failure returns error description via `AppError`.
    pub fn remove_user(&mut self, username: &str) -> Result<bool, AppError> {
        let result = self.database.lock().unwrap().remove_user(username);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(result.unwrap())
    }
    pub fn start(&mut self) {
        {
            self.logger
                .lock()
                .unwrap()
                .print_and_log(LogCategory::Info, "Starting...");
        }

        // Create socket for reporters.
        let listener_socker_reporters =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.port_for_reporters));
        if let Err(ref e) = listener_socker_reporters {
            self.logger.lock().unwrap().print_and_log(
                LogCategory::Error,
                &format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ),
            );
        }
        let listener_socker_reporters = listener_socker_reporters.unwrap();

        // Create socket for clients.
        let listener_socker_clients =
            TcpListener::bind(format!("0.0.0.0:{}", self.server_config.port_for_clients));
        if let Err(ref e) = listener_socker_clients {
            self.logger.lock().unwrap().print_and_log(
                LogCategory::Error,
                &format!(
                    "An error occurred at [{}, {}]: {:?}\n\n",
                    file!(),
                    line!(),
                    e
                ),
            );
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
        let ban_manager_clone = self.ban_manager.clone();
        thread::spawn(move || {
            NetService::process_client_connections(
                listener_socker_clients,
                logger_copy,
                connected_clone,
                database_clone,
                ban_manager_clone,
            );
        });

        let logger_guard = self.logger.lock().unwrap();
        logger_guard.print_and_log(
            LogCategory::Info,
            &format!(
                "Ready to accept client connections on port {}",
                self.server_config.port_for_clients
            ),
        );
        logger_guard.print_and_log(
            LogCategory::Info,
            &format!(
                "Ready to accept reporter connections on port {}",
                self.server_config.port_for_reporters
            ),
        );
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
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }

            let (socket, addr) = accept_result.unwrap();

            if let Err(e) = socket.set_nodelay(true) {
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }
            if let Err(e) = socket.set_nonblocking(true) {
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
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
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }
        }
    }
    fn process_client_connections(
        listener_socket: TcpListener,
        logger: Arc<Mutex<Logger>>,
        connected_count: Arc<Mutex<usize>>,
        database_manager: Arc<Mutex<DatabaseManager>>,
        ban_manager: Arc<Mutex<BanManager>>,
    ) {
        loop {
            // Wait for connection.
            let accept_result = listener_socket.accept();
            if let Err(ref e) = accept_result {
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }

            let (socket, addr) = accept_result.unwrap();

            if let Err(e) = socket.set_nodelay(true) {
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }
            if let Err(e) = socket.set_nonblocking(true) {
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }

            {
                let mut ban_manager_guard = ban_manager.lock().unwrap();

                ban_manager_guard.refresh_failed_and_banned_lists();

                // Check if this IP is banned.
                if ban_manager_guard.is_ip_banned(addr.ip()) {
                    continue;
                }
            }

            let logger_clone = logger.clone();
            let connected_count_clone = connected_count.clone();
            let database_clone = database_manager.clone();
            let ban_manager_clone = ban_manager.clone();

            let handle = thread::Builder::new()
                .name(format!("client socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new_client(
                        logger_clone,
                        socket,
                        addr,
                        connected_count_clone,
                        database_clone,
                        Some(ban_manager_clone),
                    );
                    user_service.process_client();
                });
            if let Err(ref e) = handle {
                logger.lock().unwrap().print_and_log(
                    LogCategory::Error,
                    &format!(
                        "An error occurred at [{}, {}]: {:?}\n\n",
                        file!(),
                        line!(),
                        e
                    ),
                );
            }
        }
    }
}
