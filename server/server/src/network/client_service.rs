// Std.
use std::net::*;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// External.
use chrono::{DateTime, Local};
use sha2::{Digest, Sha512};
use totp_rs::{Algorithm, TOTP};

const TOTP_ALGORITHM: Algorithm = Algorithm::SHA1; // if changed, change protocol version

// Custom.
use super::ban_manager::*;
use super::net_service::MAX_MESSAGE_SIZE_IN_BYTES_WITHOUT_ATTACHMENTS;
use crate::io::log_manager::*;
use shared::misc::db_manager::DatabaseManager;
use shared::misc::error::AppError;
use shared::network::client_packets::*;
use shared::network::messaging::*;
use shared::network::net_params::*;

const KEEP_ALIVE_CHECK_INTERVAL_MS: u64 = 60000; // 1 minute
const DISCONNECT_IF_INACTIVE_IN_SEC: u64 = 1800; // 30 minutes

pub struct ClientService {
    logger: Arc<Mutex<LogManager>>,
    database: Arc<Mutex<DatabaseManager>>,
    socket: TcpStream,
    socket_addr: SocketAddr,
    secret_key: [u8; SECRET_KEY_SIZE],
    connected_count: Arc<Mutex<usize>>,
    exit_error: Option<Result<String, AppError>>,
    ban_manager: Option<Arc<Mutex<BanManager>>>,
    username: Option<String>,
    time_of_last_received_message: DateTime<Local>,
}

impl ClientService {
    /// Creates a new client service to process client requests.
    ///
    /// ## Arguments
    /// * `logger`: log manager for logging.
    /// * `socket`: connected client socket.
    /// * `addr`: client socket address.
    /// * `connected_users_count`: shared variable that stores total connections.
    /// * `database`: database manager that handles the database.
    /// * `ban_manager`: ban manager for banning clients.
    pub fn new(
        logger: Arc<Mutex<LogManager>>,
        socket: TcpStream,
        addr: SocketAddr,
        connected_count: Arc<Mutex<usize>>,
        database: Arc<Mutex<DatabaseManager>>,
        ban_manager: Option<Arc<Mutex<BanManager>>>,
    ) -> Self {
        {
            let mut guard = connected_count.lock().unwrap();
            *guard += 1;
            logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!(
                    "accepted connection with client {}:{}\n------------------------- [connected: {}] -------------------------",
                    addr.ip(),
                    addr.port(),
                    guard
                ),
            );
        }

        let socket_addr = socket.peer_addr().unwrap();

        ClientService {
            logger,
            socket,
            connected_count,
            exit_error: None,
            secret_key: [0; SECRET_KEY_SIZE],
            database,
            ban_manager,
            username: None,
            socket_addr,
            time_of_last_received_message: Local::now(),
        }
    }

    /// Processes client requests until finished communication.
    ///
    /// After this function is finished the object should be destroyed.
    ///
    /// ## Warning
    /// Only not banned clients should be processed here.
    /// This function assumes the client is not banned.
    pub fn process(mut self) {
        let secret_key = start_establishing_secure_connection(&mut self.socket);
        if let Err(app_error) = secret_key {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            self.exit_error = Some(Err(AppError::new(
                "failed to convert Vec<u8> to generic array",
                file!(),
                line!(),
            )));
            return;
        }
        self.secret_key = result.unwrap();

        let mut is_fin = false; // don't check, react to FIN as error
        let message = receive_message(
            &mut self.socket,
            &self.secret_key,
            Some(MAX_WAIT_TIME_IN_READ_WRITE_MS),
            MAX_MESSAGE_SIZE_IN_BYTES_WITHOUT_ATTACHMENTS,
            &mut is_fin,
        );
        if let Err(e) = message {
            self.exit_error = Some(Err(e.add_entry(file!(), line!())));
            return;
        }
        let message = message.unwrap();

        // Deserialize.
        let message = bincode::deserialize::<ClientRequest>(&message);
        if let Err(e) = message {
            self.exit_error = Some(Err(AppError::new(&e.to_string(), file!(), line!())));
            return;
        }
        let message = message.unwrap();

        // Handle message.
        let result = self.handle_client_message(message);
        if let Err(app_error) = result {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        let result = result.unwrap();
        if result.is_some() {
            self.exit_error = Some(Ok(result.unwrap()));
            return;
        }

        // Connected.
        let result = self.wait_for_client_requests();
        if let Err(app_error) = result {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        let result = result.unwrap();
        if result.is_some() {
            self.exit_error = Some(Ok(result.unwrap()));
            return;
        }
    }

    /// Processes the client message.
    ///
    /// Returns `Option<String>` as `Ok`:
    /// - if `Some(String)` then there was a "soft" error
    /// (typically means that there was an error in client
    /// data (wrong credentials, protocol version, etc...))
    /// and we don't need to consider this as a bug,
    /// - if `None` then the operation finished successfully.
    ///
    /// Returns `AppError` as `Err` if there was an internal error
    /// (bug).
    fn handle_client_message(
        &mut self,
        message: ClientRequest,
    ) -> Result<Option<String>, AppError> {
        match message {
            ClientRequest::Login {
                client_net_protocol,
                username,
                password,
                otp,
            } => {
                let result = self.handle_client_login_request(
                    client_net_protocol,
                    username,
                    password,
                    otp,
                    None,
                );
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(result.unwrap())
            }
            ClientRequest::SetFirstPassword {
                client_net_protocol,
                username,
                old_password,
                new_password,
            } => {
                let result = self.handle_client_login_request(
                    client_net_protocol,
                    username,
                    old_password,
                    String::new(),
                    Some(new_password),
                );
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(result.unwrap())
            }
            ClientRequest::QueryReportsSummary { page, amount } => {
                let result = self.handle_client_reports_request(page, amount);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(None)
            }
            ClientRequest::QueryReport { report_id } => {
                let result = self.handle_client_report_request(report_id);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(None)
            }
            ClientRequest::DeleteReport { report_id } => {
                let result = self.handle_client_delete_report_request(report_id);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(None)
            }
        }
    }

    /// Processes the client login request.
    ///
    /// Returns `Option<String>` as `Ok`:
    /// - if `Some(String)` then there was a "soft" error
    /// (typically means that there was an error in client
    /// data (wrong credentials, protocol version, etc...)
    /// and we don't need to consider this as a bug,
    /// - if `None` then the operation finished successfully.
    ///
    /// Returns `AppError` as `Err` if there was an internal error
    /// (bug).
    fn handle_client_login_request(
        &mut self,
        client_net_protocol: u16,
        username: String,
        mut password: Vec<u8>,
        otp: String,
        new_password: Option<Vec<u8>>,
    ) -> Result<Option<String>, AppError> {
        // Check protocol version.
        if client_net_protocol != NETWORK_PROTOCOL_VERSION {
            let answer = ClientAnswer::LoginAnswer {
                is_ok: false,
                is_admin: false,
                fail_reason: Some(ClientLoginFailReason::WrongProtocol {
                    server_protocol: NETWORK_PROTOCOL_VERSION,
                }),
            };
            if let Some(app_error) = send_message(&mut self.socket, &self.secret_key, answer) {
                return Err(app_error.add_entry(file!(), line!()));
            }

            return Ok(Some(format!(
                "wrong protocol version ({} != {}) (username: {})",
                client_net_protocol, NETWORK_PROTOCOL_VERSION, username
            )));
        }

        // Get user's password and salt.
        let database_guard = self.database.lock().unwrap();
        let result = database_guard.get_user_password_and_salt(&username);
        drop(database_guard);

        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        // Check if user exists.
        let (db_password, salt) = result.unwrap();
        if db_password.is_empty() {
            // No user was found for this username.
            let result = self.answer_client_wrong_credentials(&username);
            if let Err(app_error) = result {
                return Err(app_error.add_entry(file!(), line!()));
            }

            return Ok(Some(result.unwrap()));
        }

        // Compare passwords.
        let mut password_to_check = Vec::from(salt.as_bytes());
        password_to_check.append(&mut password);

        // Password hash.
        let mut hasher = Sha512::new();
        hasher.update(password_to_check.as_slice());
        let password_hash: Vec<u8> = hasher.finalize().to_vec();

        if password_hash != db_password {
            // Wrong password.
            let result = self.answer_client_wrong_credentials(&username);
            if let Err(app_error) = result {
                return Err(app_error.add_entry(file!(), line!()));
            }

            return Ok(Some(result.unwrap()));
        }

        // See if user needs to set first password.
        let mut _need_change_password = false;
        {
            let result = self
                .database
                .lock()
                .unwrap()
                .is_user_needs_to_change_password(&username);
            if let Err(e) = result {
                return Err(e.add_entry(file!(), line!()));
            }
            _need_change_password = result.unwrap();
        }

        if _need_change_password && new_password.is_none() {
            // Need to set first password.
            self.logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!(
                    "{} logged in but needs to set the first password, disconnecting...",
                    &username
                ),
            );

            let answer = ClientAnswer::LoginAnswer {
                is_ok: false,
                is_admin: false,
                fail_reason: Some(ClientLoginFailReason::NeedFirstPassword),
            };
            if let Some(err) = send_message(&mut self.socket, &self.secret_key, answer) {
                return Err(err.add_entry(file!(), line!()));
            }

            return Ok(None);
        }

        if new_password.is_some() {
            // Set first password.
            let result = self
                .database
                .lock()
                .unwrap()
                .update_user_password(&username, new_password.unwrap());
            if let Err(e) = result {
                return Err(e.add_entry(file!(), line!()));
            }
            let result = result.unwrap();
            if result {
                return Ok(Some(format!(
                    "received new password from user {}, but \
                            there is no need to change the user's password",
                    username
                )));
            }

            self.logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!("{} set first password.", &username),
            );
        }

        // Check if user needs to setup OTP (receive OTP QR code).
        {
            let db_guard = self.database.lock().unwrap();
            let result = db_guard.is_user_needs_setup_otp(&username);
            if let Err(e) = result {
                return Err(e.add_entry(file!(), line!()));
            }
            let _need_setup_otp = result.unwrap();

            // Get OTP secret.
            let result = db_guard.get_otp_secret_key_for_user(&username);
            if let Err(e) = result {
                return Err(e.add_entry(file!(), line!()));
            }
            let otp_secret = result.unwrap();

            drop(db_guard);

            if _need_setup_otp && otp.is_empty() {
                // Generate QR code.
                let totp = TOTP::new(
                    TOTP_ALGORITHM,
                    6,
                    1,
                    30,
                    otp_secret,
                    Some(String::from("FBugReporter")),
                    username.clone(),
                );
                if let Err(e) = totp {
                    return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
                }
                let totp = totp.unwrap();

                let qr_code = totp.get_qr();
                if let Err(e) = qr_code {
                    return Err(AppError::new(&e.to_string(), file!(), line!()));
                }
                let qr_code = qr_code.unwrap();

                self.logger.lock().unwrap().print_and_log(
                    LogCategory::Info,
                    &format!(
                        "{} logged in but needs to setup OTP, disconnecting...",
                        &username
                    ),
                );

                // Send QR code.
                let answer = ClientAnswer::LoginAnswer {
                    is_ok: false,
                    is_admin: false,
                    fail_reason: Some(ClientLoginFailReason::SetupOTP { qr_code }),
                };
                if let Some(err) = send_message(&mut self.socket, &self.secret_key, answer) {
                    return Err(err.add_entry(file!(), line!()));
                }

                return Ok(None);
            } else {
                if otp.is_empty() {
                    // Need OTP.
                    let answer = ClientAnswer::LoginAnswer {
                        is_ok: false,
                        is_admin: false,
                        fail_reason: Some(ClientLoginFailReason::NeedOTP),
                    };
                    if let Some(err) = send_message(&mut self.socket, &self.secret_key, answer) {
                        return Err(err.add_entry(file!(), line!()));
                    }

                    return Ok(Some(format!(
                        "the user {} needs a OTP to login (usual login process, not an error)",
                        username
                    )));
                }

                // Generate current OTP.
                let totp = TOTP::new(TOTP_ALGORITHM, 6, 1, 30, otp_secret, None, String::new());
                if let Err(e) = totp {
                    return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
                }
                let totp = totp.unwrap();

                let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
                if let Err(e) = time {
                    return Err(AppError::new(&e.to_string(), file!(), line!()));
                }
                let time = time.unwrap().as_secs();
                let token = totp.generate(time);

                if token != otp {
                    self.logger.lock().unwrap().print_and_log(
                        LogCategory::Info,
                        &format!("{} tried to login using wrong OTP code.", &username),
                    );
                    let result = self.answer_client_wrong_credentials(&username);
                    if let Err(app_error) = result {
                        return Err(app_error.add_entry(file!(), line!()));
                    }

                    return Ok(Some(result.unwrap()));
                } else if _need_setup_otp {
                    let result = self
                        .database
                        .lock()
                        .unwrap()
                        .set_user_finished_otp_setup(&username);
                    if let Err(app_error) = result {
                        return Err(app_error.add_entry(file!(), line!()));
                    }
                    self.logger.lock().unwrap().print_and_log(
                        LogCategory::Info,
                        &format!("{} finished OTP setup.", &username),
                    );
                }
            }
        }

        let mut _is_admin = false;
        {
            let guard = self.database.lock().unwrap();

            // Update last login time/date/ip.
            if let Err(app_error) = guard.update_user_last_login(
                &username,
                &self.socket.peer_addr().unwrap().ip().to_string(),
            ) {
                return Err(app_error.add_entry(file!(), line!()));
            }

            // Check if user is admin.
            let result = guard.is_user_admin(&username);
            if let Err(app_error) = result {
                return Err(app_error.add_entry(file!(), line!()));
            }
            _is_admin = result.unwrap();
        }

        {
            // Remove user from failed ips.
            self.ban_manager
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .remove_ip_from_failed_ips_list(self.socket.peer_addr().unwrap().ip());
        }

        {
            // Mark user as logged in.
            self.logger
                .lock()
                .unwrap()
                .print_and_log(LogCategory::Info, &format!("{} logged in", &username));
        }

        self.username = Some(username);

        // Answer "connected".
        let answer = ClientAnswer::LoginAnswer {
            is_ok: true,
            is_admin: _is_admin,
            fail_reason: None,
        };
        if let Some(err) = send_message(&mut self.socket, &self.secret_key, answer) {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(None)
    }

    /// Handles client's "query reports" request.
    ///
    /// Will query reports and send them to the client.
    fn handle_client_reports_request(&mut self, page: u64, amount: u64) -> Result<(), AppError> {
        // Get reports from database.
        let guard = self.database.lock().unwrap();
        let result = guard.get_reports(page, amount);
        let report_count = guard.get_report_count();
        drop(guard);

        // Check reports.
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let reports = result.unwrap();

        // Check report count.
        if let Err(app_error) = report_count {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let report_count = report_count.unwrap();

        // Prepare message to send.
        let message = ClientAnswer::ReportsSummary {
            reports,
            total_reports: report_count,
        };

        // Send reports.
        let result = send_message(&mut self.socket, &self.secret_key, message);
        if let Some(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(())
    }

    /// Handles client's "delete report" request.
    ///
    /// Looks if the client has admin privileges and removes a report
    /// with the specified ID.
    fn handle_client_delete_report_request(&mut self, report_id: u64) -> Result<(), AppError> {
        // Check if this user has admin privileges.
        {
            let mut username = String::new();
            if self.username.is_some() {
                username = self.username.as_ref().unwrap().clone();
            }
            let result = self.database.lock().unwrap().is_user_admin(&username);
            if let Err(e) = result {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let is_admin = result.unwrap();

            if is_admin {
                self.logger.lock().unwrap().print_and_log(
                    LogCategory::Info,
                    &format!(
                        "admin client \"{}\" requested to delete a report with id {}",
                        &username, report_id
                    ),
                )
            } else {
                let message = format!(
                    "client \"{}\" tried to \
                    delete a report with id {} without admin privileges",
                    &username, is_admin
                );
                self.logger
                    .lock()
                    .unwrap()
                    .print_and_log(LogCategory::Warning, &message);
                return Err(AppError::new(&message, file!(), line!()));
            }
        }

        // Remove report from database.
        let result = self.database.lock().unwrap().remove_report(report_id);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let found = result.unwrap();
        if !found {
            let mut username = String::new();
            if self.username.is_some() {
                username = self.username.as_ref().unwrap().clone();
            }
            self.logger.lock().unwrap().print_and_log(
                LogCategory::Warning,
                &format!(
                    "admin client \"{}\" tried to \
                    delete a report with id {} while a report with this id does not exist",
                    &username, report_id
                ),
            );
        }

        // Prepare message to send.
        let message = ClientAnswer::DeleteReportResult {
            is_found_and_removed: found,
        };

        // Send reports.
        let result = send_message(&mut self.socket, &self.secret_key, message);
        if let Some(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(())
    }

    /// Handles client's "query report" request.
    ///
    /// Queries the specified report from the database and returns
    /// it to the client.
    fn handle_client_report_request(&mut self, report_id: u64) -> Result<(), AppError> {
        {
            // Log this event.
            let mut username = String::new();
            if self.username.is_some() {
                username = self.username.as_ref().unwrap().clone();
            }
            self.logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!(
                    "client \"{}\" requested a report with id {}",
                    username, report_id
                ),
            )
        }

        // Get reports from database.
        let guard = self.database.lock().unwrap();
        let result = guard.get_report(report_id);
        drop(guard);

        // Check report.
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let report = result.unwrap();

        // Prepare message to send.
        let message = ClientAnswer::Report {
            id: report.id,
            title: report.title,
            game_name: report.game_name,
            game_version: report.game_version,
            text: report.text,
            date: report.date,
            time: report.time,
            sender_name: report.sender_name,
            sender_email: report.sender_email,
            os_info: report.os_info,
        };

        // Send reports.
        let result = send_message(&mut self.socket, &self.secret_key, message);
        if let Some(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(())
    }

    /// Sends `ClientLoginAnswer` with `WrongCredentials` message
    /// to the client.
    ///
    /// Returns `String` as `Ok` with message to show
    /// (i.e. "wrong credentials for username ...").
    ///
    /// Returns `AppError` as `Err` if there was an internal error
    /// (bug).
    fn answer_client_wrong_credentials(&mut self, username: &str) -> Result<String, AppError> {
        let mut _result = AttemptResult::Ban;
        {
            _result = self
                .ban_manager
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .add_failed_login_attempt(username, self.socket.peer_addr().unwrap().ip());
        }

        match _result {
            AttemptResult::Fail { attempts_made } => {
                let _answer = ClientAnswer::LoginAnswer {
                    is_ok: false,
                    is_admin: false,
                    fail_reason: Some(ClientLoginFailReason::WrongCredentials {
                        result: ClientLoginFailResult::FailedAttempt {
                            failed_attempts_made: attempts_made,
                            max_failed_attempts: self
                                .ban_manager
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .config
                                .max_allowed_login_attempts,
                        },
                    }),
                };
                if let Some(err) = send_message(&mut self.socket, &self.secret_key, _answer) {
                    return Err(err.add_entry(file!(), line!()));
                }
            }
            AttemptResult::Ban => {
                let _answer = ClientAnswer::LoginAnswer {
                    is_ok: false,
                    is_admin: false,
                    fail_reason: Some(ClientLoginFailReason::WrongCredentials {
                        result: ClientLoginFailResult::Banned {
                            ban_time_in_min: self
                                .ban_manager
                                .as_ref()
                                .unwrap()
                                .lock()
                                .unwrap()
                                .config
                                .ban_time_duration_in_min,
                        },
                    }),
                };
                if let Some(err) = send_message(&mut self.socket, &self.secret_key, _answer) {
                    return Err(err.add_entry(file!(), line!()));
                }
            }
        }

        return Ok(format!("wrong credentials (login username: {})", username));
    }
    /// Waits for new requests from the client.
    ///
    /// Returns `Option<String>` as `Ok`:
    /// - if `Some(String)` then there was a "soft" error
    /// (typically means that there was an error in client
    /// data (out of bounds page requested, non-existent report requested and etc.))
    /// and we don't need to consider this as a bug,
    /// - if `None` then the operation finished successfully.
    ///
    /// Returns `AppError` as `Err` if there was an internal error
    /// (bug).
    fn wait_for_client_requests(&mut self) -> Result<Option<String>, AppError> {
        let mut is_fin = false;

        self.time_of_last_received_message = Local::now();

        loop {
            let result = receive_message(
                &mut self.socket,
                &self.secret_key,
                Some(KEEP_ALIVE_CHECK_INTERVAL_MS),
                MAX_MESSAGE_SIZE_IN_BYTES_WITHOUT_ATTACHMENTS,
                &mut is_fin,
            );
            if is_fin {
                return Ok(None);
            }
            if let Err(app_error) = result {
                return Err(app_error.add_entry(file!(), line!()));
            }
            let message = result.unwrap();

            if message.is_empty() {
                // Timeout.
                let result = self.check_client_keep_alive();
                if let Err(message) = result {
                    return Ok(Some(message));
                }
                continue;
            } else {
                self.time_of_last_received_message = Local::now();
            }

            // Deserialize.
            let message = bincode::deserialize::<ClientRequest>(&message);
            if let Err(e) = message {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let message = message.unwrap();

            // Handle message.
            let result = self.handle_client_message(message);
            if let Err(app_error) = result {
                return Err(app_error.add_entry(file!(), line!()));
            }
            let result = result.unwrap();
            if result.is_some() {
                return Ok(result);
            }
        }
    }
    /// Checks if the connection is not lost.
    ///
    /// Returns `Ok(())` if connection is not lost,
    /// returns `Err(String)` if the connection was lost
    /// (contains connection lost message).
    fn check_client_keep_alive(&mut self) -> Result<(), String> {
        let time_diff = Local::now() - self.time_of_last_received_message;

        if time_diff.num_seconds() >= DISCONNECT_IF_INACTIVE_IN_SEC as i64 {
            // Disconnect.
            if self.username.is_some() {
                return Err(format!(
                    "disconnecting user \"{}\" due to inactivity for {} second(-s)",
                    self.username.as_ref().unwrap(),
                    DISCONNECT_IF_INACTIVE_IN_SEC
                ));
            } else {
                return Err(format!(
                    "disconnecting socket {} due to inactivity for {} second(-s)",
                    self.socket_addr, DISCONNECT_IF_INACTIVE_IN_SEC
                ));
            }
        }

        Ok(())
    }
}

impl Drop for ClientService {
    /// Logs information about connection being closed.
    fn drop(&mut self) {
        let mut _message = String::new();

        if self.username.is_some() {
            _message = format!("{} logged out", self.username.as_ref().unwrap());
        } else {
            _message = format!("closing connection with client {}", self.socket_addr);
        }

        if self.exit_error.is_some() {
            let error = self.exit_error.as_ref().unwrap();

            if let Err(app_error) = error {
                _message += &format!(" due to internal error (bug):\n{}", app_error);
            } else {
                _message += &format!(", reason: {}", error.as_ref().unwrap());
            }
        }

        _message += "\n";

        let mut guard = self.connected_count.lock().unwrap();
        *guard -= 1;
        _message += &format!(
            "------------------------- [connected: {}] -------------------------",
            guard
        );

        self.logger
            .lock()
            .unwrap()
            .print_and_log(LogCategory::Info, &_message);
    }
}
