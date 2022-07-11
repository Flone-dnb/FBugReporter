// Std.
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;

// Custom.
use crate::{
    io::config_manager::ConfigManager,
    io::log_manager::*,
    network::{ban_manager::BanManager, user_service::UserService},
};
use shared::misc::db_manager::*;
use shared::misc::error::AppError;

pub struct NetService {
    pub logger: Arc<Mutex<LogManager>>,
    pub server_config: Arc<ConfigManager>,
    connected_socket_count: Arc<Mutex<usize>>,
    database: Arc<Mutex<DatabaseManager>>,
    ban_manager: Arc<Mutex<BanManager>>,
}

impl NetService {
    /// Creates a new instance of the `NetService`.
    ///
    /// Returns `AppError` if something went wrong
    /// when initializing/connecting to the database.
    pub fn new(logger: LogManager) -> Result<Self, AppError> {
        let config = Arc::new(ConfigManager::new());

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
    /// Starts waiting for client and reporter requests.
    pub fn start(&mut self, blocking: bool) {
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

        {
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

        // Process reporters.
        let logger_copy = self.logger.clone();
        let connected_clone = self.connected_socket_count.clone();
        let database_clone = self.database.clone();
        let config_clone = self.server_config.clone();
        let reporter_handle = thread::spawn(move || {
            NetService::process_reporter_connections(
                listener_socker_reporters,
                logger_copy,
                connected_clone,
                database_clone,
                config_clone,
            );
        });

        // Process clients.
        let logger_copy = self.logger.clone();
        let connected_clone = self.connected_socket_count.clone();
        let database_clone = self.database.clone();
        let ban_manager_clone = self.ban_manager.clone();
        let client_handle = thread::spawn(move || {
            NetService::process_client_connections(
                listener_socker_clients,
                logger_copy,
                connected_clone,
                database_clone,
                ban_manager_clone,
            );
        });

        if blocking {
            reporter_handle.join().unwrap();
            client_handle.join().unwrap();
        }
    }
    /// Waits for reporter connections.
    fn process_reporter_connections(
        listener_socket: TcpListener,
        logger: Arc<Mutex<LogManager>>,
        connected_count: Arc<Mutex<usize>>,
        database_manager: Arc<Mutex<DatabaseManager>>,
        server_config: Arc<ConfigManager>,
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
            let max_attachment_size_in_mb = server_config.max_attachment_size_in_mb;

            let handle = thread::Builder::new()
                .name(format!("reporter socket {}:{}", addr.ip(), addr.port()))
                .spawn(move || {
                    let mut user_service = UserService::new_reporter(
                        logger_copy,
                        socket,
                        addr,
                        connected_count_clone,
                        database_clone,
                        max_attachment_size_in_mb,
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
    /// Waits for client connections.
    fn process_client_connections(
        listener_socket: TcpListener,
        logger: Arc<Mutex<LogManager>>,
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
