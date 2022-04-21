// Std.
use std::io::prelude::*;
use std::net::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

// External.
use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use chrono::{DateTime, Local};
use cmac::{Cmac, Mac, NewMac};
use num_bigint::{BigUint, RandomBits};
use rand::{Rng, RngCore};
use serde::Serialize;
use sha2::{Digest, Sha512};
use totp_rs::{Algorithm, TOTP};

// Custom.
use super::ban_manager::*;
use super::net_packets::*;
use crate::misc::*;
use crate::services::logger_service::*;
use shared::db_manager::DatabaseManager;
use shared::error::AppError;
use shared::report::GameReport;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;
const KEY_LENGTH_IN_BYTES: usize = 32; // if changed, change protocol version

const A_B_BITS: u64 = 2048; // if changed, change protocol version
const IV_LENGTH: usize = 16; // if changed, change protocol version
const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version
const NETWORK_PROTOCOL_VERSION: u16 = 0;
const MAX_PACKET_SIZE_IN_BYTES: u32 = 131_072; // 128 kB for now
const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 25;
const MAX_WAIT_TIME_IN_READ_WRITE_MS: u64 = 5000;
const KEEP_ALIVE_CHECK_INTERVAL_MS: u64 = 60000; // 1 minute
const DISCONNECT_IF_INACTIVE_IN_SEC: u64 = 1800; // 30 minutes

enum IoResult {
    Ok(usize),
    Fin,
    Err(AppError),
}

pub struct UserService {
    logger: Arc<Mutex<Logger>>,
    socket: TcpStream,
    socket_addr: Option<SocketAddr>,
    secret_key: Vec<u8>,
    connected_users_count: Arc<Mutex<usize>>,
    exit_error: Option<Result<String, AppError>>,
    database: Arc<Mutex<DatabaseManager>>,
    ban_manager: Option<Arc<Mutex<BanManager>>>,
    username: Option<String>,
    time_of_last_received_packet: DateTime<Local>, // only for clients
}

impl UserService {
    /// Creates a new client service to process client requests.
    pub fn new_client(
        logger: Arc<Mutex<Logger>>,
        socket: TcpStream,
        addr: SocketAddr,
        connected_users_count: Arc<Mutex<usize>>,
        database: Arc<Mutex<DatabaseManager>>,
        ban_manager: Option<Arc<Mutex<BanManager>>>,
    ) -> Self {
        {
            let mut guard = connected_users_count.lock().unwrap();
            *guard += 1;
            logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!(
                    "Accepted connection with {}:{}\n--- [connected: {}] ---",
                    addr.ip(),
                    addr.port(),
                    guard
                ),
            );
        }

        UserService {
            logger,
            socket,
            connected_users_count,
            exit_error: None,
            secret_key: Vec::new(),
            database,
            ban_manager,
            username: None,
            socket_addr: None,
            time_of_last_received_packet: Local::now(),
        }
    }
    /// Creates a new reporter service to process reporter requests.
    pub fn new_reporter(
        logger: Arc<Mutex<Logger>>,
        socket: TcpStream,
        addr: SocketAddr,
        connected_users_count: Arc<Mutex<usize>>,
        database: Arc<Mutex<DatabaseManager>>,
    ) -> Self {
        {
            let mut guard = connected_users_count.lock().unwrap();
            *guard += 1;
            logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!(
                    "Accepted connection with {}:{}\n--- [connected: {}] ---",
                    addr.ip(),
                    addr.port(),
                    guard
                ),
            );
        }

        UserService {
            logger,
            socket,
            connected_users_count,
            exit_error: None,
            secret_key: Vec::new(),
            database,
            ban_manager: None,
            socket_addr: None,
            username: None,
            time_of_last_received_packet: Local::now(),
        }
    }
    /// Processes reporter requests.
    ///
    /// After this function is finished the object should be destroyed.
    pub fn process_reporter(&mut self) {
        let secret_key = UserService::establish_secure_connection(&mut self.socket);
        if let Err(app_error) = secret_key {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        self.secret_key = secret_key.unwrap();

        let mut is_fin = false; // don't check, react to FIN as error
        let packet = self.receive_packet(&mut is_fin, None);
        if let Err(app_error) = packet {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        let packet = packet.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<InReporterPacket>(&packet);
        if let Err(e) = packet {
            self.exit_error = Some(Err(AppError::new(&e.to_string(), file!(), line!())));
            return;
        }
        let packet = packet.unwrap();

        self.socket_addr = Some(self.socket.peer_addr().unwrap());

        let result = self.handle_reporter_packet(packet);
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
    /// Processes client requests.
    ///
    /// After this function is finished the object should be destroyed.
    ///
    /// # Warning
    /// Only not banned clients should be processed here.
    /// This function assumes the client is not banned.
    pub fn process_client(&mut self) {
        let secret_key = UserService::establish_secure_connection(&mut self.socket);
        if let Err(app_error) = secret_key {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        self.secret_key = secret_key.unwrap();

        let mut is_fin = false; // don't check, react to FIN as error
        let packet = self.receive_packet(&mut is_fin, None);
        if let Err(app_error) = packet {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        let packet = packet.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<InClientPacket>(&packet);
        if let Err(e) = packet {
            self.exit_error = Some(Err(AppError::new(&e.to_string(), file!(), line!())));
            return;
        }
        let packet = packet.unwrap();

        // Handle packet.
        let result = self.handle_client_packet(packet);
        if let Err(app_error) = result {
            self.exit_error = Some(Err(app_error.add_entry(file!(), line!())));
            return;
        }
        let result = result.unwrap();
        if result.is_some() {
            self.exit_error = Some(Ok(result.unwrap()));
            return;
        }

        // Save address.
        self.socket_addr = Some(self.socket.peer_addr().unwrap());

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
    /// Processes the client packet.
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
    fn handle_reporter_packet(
        &mut self,
        packet: InReporterPacket,
    ) -> Result<Option<String>, AppError> {
        match packet {
            InReporterPacket::ReportPacket {
                reporter_net_protocol,
                game_report,
            } => {
                // Check protocol version.
                if reporter_net_protocol != NETWORK_PROTOCOL_VERSION {
                    let result_code = ReportResult::WrongProtocol;

                    // Notify reporter.
                    if let Err(err) = UserService::send_packet(
                        &mut self.socket,
                        &self.secret_key,
                        OutReporterPacket::ReportAnswer { result_code },
                    ) {
                        return Err(err.add_entry(file!(), line!()));
                    }

                    return Ok(Some(format!(
                        "wrong protocol version (reporter: {}, our: {})",
                        reporter_net_protocol, NETWORK_PROTOCOL_VERSION
                    )));
                }

                // Check field limits.
                if let Err((field, length)) = UserService::check_report_field_limits(&game_report) {
                    let result_code = ReportResult::ServerRejected;

                    // Notify reporter.
                    if let Err(err) = UserService::send_packet(
                        &mut self.socket,
                        &self.secret_key,
                        OutReporterPacket::ReportAnswer { result_code },
                    ) {
                        return Err(err.add_entry(file!(), line!()));
                    }

                    return Ok(Some(format!(
                        "report exceeds report field limits ({:?} has length of {} characters \
                    while the limit is {})",
                        field,
                        length,
                        field.max_length()
                    )));
                }

                self.logger.lock().unwrap().print_and_log(
                    LogCategory::Info,
                    &format!(
                        "Received a report from socket {}",
                        self.socket_addr.unwrap()
                    ),
                );

                {
                    if let Err(err) = self.database.lock().unwrap().save_report(game_report) {
                        let result_code = ReportResult::InternalError;

                        // Notify reporter of our failure.
                        if let Err(err) = UserService::send_packet(
                            &mut self.socket,
                            &self.secret_key,
                            OutReporterPacket::ReportAnswer { result_code },
                        ) {
                            return Err(err.add_entry(file!(), line!()));
                        }

                        return Err(err.add_entry(file!(), line!()));
                    }
                }

                self.logger.lock().unwrap().print_and_log(
                    LogCategory::Info,
                    &format!(
                        "Saved a report from socket {}",
                        match self.socket_addr {
                            Some(addr) => {
                                addr.to_string()
                            }
                            None => {
                                String::new()
                            }
                        }
                    ),
                );

                // Answer "OK".
                if let Err(err) = UserService::send_packet(
                    &mut self.socket,
                    &self.secret_key,
                    OutReporterPacket::ReportAnswer {
                        result_code: ReportResult::Ok,
                    },
                ) {
                    return Err(err.add_entry(file!(), line!()));
                }
            }
        }

        Ok(None)
    }
    /// Processes the client packet.
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
    fn handle_client_packet(&mut self, packet: InClientPacket) -> Result<Option<String>, AppError> {
        match packet {
            InClientPacket::Login {
                client_net_protocol,
                username,
                password,
                otp,
            } => {
                let result =
                    self.handle_client_login(client_net_protocol, username, password, otp, None);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(result.unwrap())
            }
            InClientPacket::SetFirstPassword {
                client_net_protocol,
                username,
                old_password,
                new_password,
            } => {
                let result = self.handle_client_login(
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
            InClientPacket::QueryReportsSummary { page, amount } => {
                let result = self.handle_client_reports_request(page, amount);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(None)
            }
            InClientPacket::QueryReport { report_id } => {
                let result = self.handle_client_report_request(report_id);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(None)
            }
            InClientPacket::DeleteReport { report_id } => {
                let result = self.handle_client_delete_report_request(report_id);
                if let Err(app_error) = result {
                    return Err(app_error.add_entry(file!(), line!()));
                }

                Ok(None)
            }
        }
    }
    /// Processes the client login packet.
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
    fn handle_client_login(
        &mut self,
        client_net_protocol: u16,
        username: String,
        mut password: Vec<u8>,
        otp: String,
        new_password: Option<Vec<u8>>,
    ) -> Result<Option<String>, AppError> {
        // Check protocol version.
        if client_net_protocol != NETWORK_PROTOCOL_VERSION {
            let answer = OutClientPacket::LoginAnswer {
                is_ok: false,
                is_admin: false,
                fail_reason: Some(ClientLoginFailReason::WrongProtocol {
                    server_protocol: NETWORK_PROTOCOL_VERSION,
                }),
            };
            if let Err(app_error) =
                UserService::send_packet(&mut self.socket, &self.secret_key, answer)
            {
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

            let answer = OutClientPacket::LoginAnswer {
                is_ok: false,
                is_admin: false,
                fail_reason: Some(ClientLoginFailReason::NeedFirstPassword),
            };
            if let Err(err) = UserService::send_packet(&mut self.socket, &self.secret_key, answer) {
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
                let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, otp_secret);
                let qr_code = totp.get_qr("FBugReporter", &username);
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
                let answer = OutClientPacket::LoginAnswer {
                    is_ok: false,
                    is_admin: false,
                    fail_reason: Some(ClientLoginFailReason::SetupOTP { qr_code }),
                };
                if let Err(err) =
                    UserService::send_packet(&mut self.socket, &self.secret_key, answer)
                {
                    return Err(err.add_entry(file!(), line!()));
                }

                return Ok(None);
            } else {
                if otp.is_empty() {
                    // Need OTP.
                    let answer = OutClientPacket::LoginAnswer {
                        is_ok: false,
                        is_admin: false,
                        fail_reason: Some(ClientLoginFailReason::NeedOTP),
                    };
                    if let Err(err) =
                        UserService::send_packet(&mut self.socket, &self.secret_key, answer)
                    {
                        return Err(err.add_entry(file!(), line!()));
                    }

                    return Ok(Some(format!(
                        "the user {} needs a OTP to login (usual login process, not an error)",
                        username
                    )));
                }

                // Generate current OTP.
                let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, otp_secret);
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
        let answer = OutClientPacket::LoginAnswer {
            is_ok: true,
            is_admin: _is_admin,
            fail_reason: None,
        };
        if let Err(err) = UserService::send_packet(&mut self.socket, &self.secret_key, answer) {
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

        // Prepare packet to send.
        let packet = OutClientPacket::ReportsSummary {
            reports,
            total_reports: report_count,
        };

        // Send reports.
        let result = UserService::send_packet(&mut self.socket, &self.secret_key, packet);
        if let Err(app_error) = result {
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
                        "admin user '{}' requested to delete a report with id {}",
                        &username, report_id
                    ),
                )
            } else {
                let message = format!(
                    "user '{}' tried to \
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
        if found == false {
            let mut username = String::new();
            if self.username.is_some() {
                username = self.username.as_ref().unwrap().clone();
            }
            self.logger.lock().unwrap().print_and_log(
                LogCategory::Warning,
                &format!(
                    "admin user '{}' tried to \
                    delete a report with id {} while a report with this id does not exist",
                    &username, report_id
                ),
            );
        }

        // Prepare packet to send.
        let packet = OutClientPacket::DeleteReportResult {
            is_found_and_removed: found,
        };

        // Send reports.
        let result = UserService::send_packet(&mut self.socket, &self.secret_key, packet);
        if let Err(app_error) = result {
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
                    "user '{}' requested a report with id {}",
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

        // Prepare packet to send.
        let packet = OutClientPacket::Report {
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
        let result = UserService::send_packet(&mut self.socket, &self.secret_key, packet);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        Ok(())
    }
    /// Sends `ClientLoginAnswer` with `WrongCredentials` packet
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
                let _answer = OutClientPacket::LoginAnswer {
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
                if let Err(err) =
                    UserService::send_packet(&mut self.socket, &self.secret_key, _answer)
                {
                    return Err(err.add_entry(file!(), line!()));
                }
            }
            AttemptResult::Ban => {
                let _answer = OutClientPacket::LoginAnswer {
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
                if let Err(err) =
                    UserService::send_packet(&mut self.socket, &self.secret_key, _answer)
                {
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

        self.time_of_last_received_packet = Local::now();

        loop {
            let result = self.receive_packet(&mut is_fin, Some(KEEP_ALIVE_CHECK_INTERVAL_MS));
            if is_fin {
                return Ok(None);
            }
            if let Err(app_error) = result {
                return Err(app_error.add_entry(file!(), line!()));
            }
            let packet = result.unwrap();

            if packet.is_empty() {
                // Timeout.
                let result = self.check_client_keep_alive();
                if let Err(message) = result {
                    return Ok(Some(message));
                }
                continue;
            } else {
                self.time_of_last_received_packet = Local::now();
            }

            // Deserialize.
            let packet = bincode::deserialize::<InClientPacket>(&packet);
            if let Err(e) = packet {
                return Err(AppError::new(&e.to_string(), file!(), line!()));
            }
            let packet = packet.unwrap();

            // Handle packet.
            let result = self.handle_client_packet(packet);
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
        let time_diff = Local::now() - self.time_of_last_received_packet;

        if time_diff.num_seconds() >= DISCONNECT_IF_INACTIVE_IN_SEC as i64 {
            // Disconnect.
            if self.username.is_some() {
                return Err(format!(
                    "disconnecting user '{}' due to inactivity for {} second(-s)",
                    self.username.as_ref().unwrap(),
                    DISCONNECT_IF_INACTIVE_IN_SEC
                ));
            } else {
                return Err(format!(
                    "disconnecting socket {} due to inactivity for {} second(-s)",
                    match self.socket_addr {
                        Some(addr) => {
                            addr.to_string()
                        }
                        None => {
                            String::new()
                        }
                    },
                    DISCONNECT_IF_INACTIVE_IN_SEC
                ));
            }
        }

        Ok(())
    }
    /// Generates a secret key that will be used to encrypt network packets.
    ///
    /// Returns `Ok(Vec<u8>)` with the secret key if no errors occurred.
    fn establish_secure_connection(socket: &mut TcpStream) -> Result<Vec<u8>, AppError> {
        // taken from https://www.rfc-editor.org/rfc/rfc5114#section-2.1
        let p = BigUint::parse_bytes(
            b"B10B8F96A080E01DDE92DE5EAE5D54EC52C99FBCFB06A3C69A6A9DCA52D23B616073E28675A23D189838EF1E2EE652C013ECB4AEA906112324975C3CD49B83BFACCBDD7D90C4BD7098488E9C219A73724EFFD6FAE5644738FAA31A4FF55BCCC0A151AF5F0DC8B4BD45BF37DF365C1A65E68CFDA76D4DA708DF1FB2BC2E4A4371",
            16
        ).unwrap();
        let g = BigUint::parse_bytes(
            b"A4D1CBD5C3FD34126765A442EFB99905F8104DD258AC507FD6406CFF14266D31266FEA1E5C41564B777E690F5504F213160217B4B01B886A5E91547F9E2749F4D7FBD7D3B9A92EE1909D0D2263F80A76A6A24C087A091F531DBF0A0169B6A28AD662A4D18E73AFA32D779D5918D08BC8858F4DCEF97C2A24855E6EEB22B3B2E5",
            16
        ).unwrap();

        // Send 2 values: p (BigUint), g (BigUint) values.
        let p_buf = bincode::serialize(&p);
        let g_buf = bincode::serialize(&g);

        if let Err(e) = p_buf {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let mut p_buf = p_buf.unwrap();

        if let Err(e) = g_buf {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let mut g_buf = g_buf.unwrap();

        let p_len = p_buf.len() as u64;
        let mut p_len = bincode::serialize(&p_len).unwrap();

        let g_len = g_buf.len() as u64;
        let mut g_len = bincode::serialize(&g_len).unwrap();

        let mut pg_send_buf = Vec::new();
        pg_send_buf.append(&mut p_len);
        pg_send_buf.append(&mut p_buf);
        pg_send_buf.append(&mut g_len);
        pg_send_buf.append(&mut g_buf);

        // Send p and g values.
        loop {
            match UserService::write_to_socket(socket, &mut pg_send_buf, true) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        // Generate secret key 'a'.
        let mut rng = rand::thread_rng();
        let a: BigUint = rng.sample(RandomBits::new(A_B_BITS));

        // Generate open key 'A'.
        let a_open = g.modpow(&a, &p);

        // Prepare to send open key 'A'.
        let a_open_buf = bincode::serialize(&a_open);
        if let Err(e) = a_open_buf {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let mut a_open_buf = a_open_buf.unwrap();

        // Send open key 'A'.
        let a_open_len = a_open_buf.len() as u64;
        let a_open_len_buf = bincode::serialize(&a_open_len);
        if let Err(e) = a_open_len_buf {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let mut a_open_len_buf = a_open_len_buf.unwrap();
        a_open_len_buf.append(&mut a_open_buf);
        loop {
            match UserService::write_to_socket(socket, &mut a_open_len_buf, true) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        // Receive open key 'B' size.
        let mut b_open_len_buf = vec![0u8; std::mem::size_of::<u64>()];
        loop {
            match UserService::read_from_socket(socket, &mut b_open_len_buf) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        // Receive open key 'B'.
        let b_open_len = bincode::deserialize::<u64>(&b_open_len_buf);
        if let Err(e) = b_open_len {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let b_open_len = b_open_len.unwrap();
        let mut b_open_buf = vec![0u8; b_open_len as usize];

        loop {
            match UserService::read_from_socket(socket, &mut b_open_buf) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => {
                    return Err(err.add_entry(file!(), line!()));
                }
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        let b_open_big = bincode::deserialize::<BigUint>(&b_open_buf);
        if let Err(e) = b_open_big {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let b_open_big = b_open_big.unwrap();

        // Calculate the secret key.
        let secret_key = b_open_big.modpow(&a, &p);
        let mut secret_key_str = secret_key.to_str_radix(10);

        if secret_key_str.len() < KEY_LENGTH_IN_BYTES {
            if secret_key_str.is_empty() {
                return Err(AppError::new(
                    "generated secret key is empty",
                    file!(),
                    line!(),
                ));
            }

            loop {
                secret_key_str += &secret_key_str.clone();

                if secret_key_str.len() >= KEY_LENGTH_IN_BYTES {
                    break;
                }
            }
        }

        Ok(Vec::from(&secret_key_str[0..KEY_LENGTH_IN_BYTES]))
    }
    /// Returns [`Ok`] if the fields have the correct length (amount of characters, not byte count),
    /// otherwise returns the field type and its received length (not the limit, actual length).
    fn check_report_field_limits(report: &GameReport) -> Result<(), (ReportLimits, usize)> {
        if report.report_name.chars().count() > ReportLimits::ReportName.max_length() {
            return Err((ReportLimits::ReportName, report.report_name.chars().count()));
        }

        if report.report_text.chars().count() > ReportLimits::ReportText.max_length() {
            return Err((ReportLimits::ReportText, report.report_text.chars().count()));
        }

        if report.sender_name.chars().count() > ReportLimits::SenderName.max_length() {
            return Err((ReportLimits::SenderName, report.sender_name.chars().count()));
        }

        if report.sender_email.chars().count() > ReportLimits::SenderEMail.max_length() {
            return Err((
                ReportLimits::SenderEMail,
                report.sender_email.chars().count(),
            ));
        }

        if report.game_name.chars().count() > ReportLimits::GameName.max_length() {
            return Err((ReportLimits::GameName, report.game_name.chars().count()));
        }

        if report.game_version.chars().count() > ReportLimits::GameVersion.max_length() {
            return Err((
                ReportLimits::GameVersion,
                report.game_version.chars().count(),
            ));
        }

        Ok(())
    }
    /// Encrypts and sends a packet.
    ///
    /// Usually packet is `OutClientPacket` or `OutReporterPacket`,
    /// see `net_packets.rs`.
    fn send_packet<T>(socket: &mut TcpStream, secret_key: &[u8], packet: T) -> Result<(), AppError>
    where
        T: Serialize,
    {
        if secret_key.is_empty() {
            return Err(AppError::new(
                "can't send packet - secure connected is not established",
                file!(),
                line!(),
            ));
        }

        // Serialize.
        let mut binary_packet = bincode::serialize(&packet).unwrap();

        // CMAC.
        let mut mac = Cmac::<Aes256>::new_from_slice(&secret_key).unwrap();
        mac.update(&binary_packet);
        let result = mac.finalize();
        let mut tag_bytes = result.into_bytes().to_vec();
        if tag_bytes.len() != CMAC_TAG_LENGTH {
            return Err(AppError::new(
                &format!(
                    "unexpected tag length: {} != {}",
                    tag_bytes.len(),
                    CMAC_TAG_LENGTH
                ),
                file!(),
                line!(),
            ));
        }

        binary_packet.append(&mut tag_bytes);

        // Encrypt packet.
        let mut rng = rand::thread_rng();
        let mut iv = vec![0u8; IV_LENGTH];
        rng.fill_bytes(&mut iv);
        let cipher = Aes256Cbc::new_from_slices(&secret_key, &iv).unwrap();
        let mut encrypted_packet = cipher.encrypt_vec(&binary_packet);

        // Prepare encrypted packet len buffer.
        if encrypted_packet.len() + IV_LENGTH > std::u32::MAX as usize {
            // should never happen
            return Err(AppError::new(
                &format!(
                    "resulting packet is too big ({} > {})",
                    encrypted_packet.len() + IV_LENGTH,
                    std::u32::MAX
                ),
                file!(),
                line!(),
            ));
        }
        let encrypted_len = (encrypted_packet.len() + IV_LENGTH) as u32;
        let encrypted_len_buf = bincode::serialize(&encrypted_len);
        if let Err(e) = encrypted_len_buf {
            return Err(AppError::new(&format!("{:?}", e), file!(), line!()));
        }
        let mut send_buffer = encrypted_len_buf.unwrap();

        // Merge all to one buffer.
        send_buffer.append(&mut iv);
        send_buffer.append(&mut encrypted_packet);

        // Send.
        loop {
            match UserService::write_to_socket(socket, &mut send_buffer, true) {
                IoResult::Fin => {
                    return Err(AppError::new("unexpected FIN received", file!(), line!()));
                }
                IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
                IoResult::Ok(_) => {
                    break;
                }
            }
        }

        Ok(())
    }
    /// Waits for next packet to arrive.
    ///
    /// Parameters:
    /// - `is_fin`: will be `true` if the remove socket closed connection.
    /// - `timeout_in_ms`: if specified will be used instead of the default timeout,
    /// if we reached timeout and this parameter was not `None` will return `Ok` with
    /// zero length vector.
    fn receive_packet(
        &mut self,
        is_fin: &mut bool,
        timeout_in_ms: Option<u64>,
    ) -> Result<Vec<u8>, AppError> {
        if self.secret_key.is_empty() {
            return Err(AppError::new(
                "can't receive packet - secure connected is not established",
                file!(),
                line!(),
            ));
        }

        // Read u32 (size of a packet)
        let mut packet_size_buf = [0u8; std::mem::size_of::<u32>() as usize];
        let mut _next_packet_size: u32 = 0;

        let mut _result = IoResult::Fin;
        if timeout_in_ms.is_some() {
            let timeout_result = UserService::read_from_socket_with_timeout(
                &mut self.socket,
                &mut packet_size_buf,
                timeout_in_ms.unwrap(),
            );
            if timeout_result.is_none() {
                return Ok(Vec::new());
            } else {
                _result = timeout_result.unwrap();
            }
        } else {
            _result = UserService::read_from_socket(&mut self.socket, &mut packet_size_buf);
        }

        match _result {
            IoResult::Fin => {
                *is_fin = true;
                return Err(AppError::new(
                    &format!(
                        "unexpected FIN received (socket: {})",
                        match self.socket_addr {
                            Some(addr) => {
                                addr.to_string()
                            }
                            None => {
                                String::new()
                            }
                        }
                    ),
                    file!(),
                    line!(),
                ));
            }
            IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
            IoResult::Ok(byte_count) => {
                if byte_count != packet_size_buf.len() {
                    return Err(AppError::new(
                        &format!(
                            "not all data received (got: {}, expected: {}) (socket: {})",
                            byte_count,
                            packet_size_buf.len(),
                            match self.socket_addr {
                                Some(addr) => {
                                    addr.to_string()
                                }
                                None => {
                                    String::new()
                                }
                            }
                        ),
                        file!(),
                        line!(),
                    ));
                }

                let res = bincode::deserialize(&packet_size_buf);
                if let Err(e) = res {
                    return Err(AppError::new(
                        &format!(
                            "{:?} (socket: {})",
                            e,
                            match self.socket_addr {
                                Some(addr) => {
                                    addr.to_string()
                                }
                                None => {
                                    String::new()
                                }
                            }
                        ),
                        file!(),
                        line!(),
                    ));
                }

                _next_packet_size = res.unwrap();
            }
        }

        // Check packet size.
        if _next_packet_size > MAX_PACKET_SIZE_IN_BYTES {
            return Err(AppError::new(
                &format!(
                    "incoming packet is too big to receive ({} > {} bytes) (socket: {})",
                    _next_packet_size,
                    MAX_PACKET_SIZE_IN_BYTES,
                    match self.socket_addr {
                        Some(addr) => {
                            addr.to_string()
                        }
                        None => {
                            String::new()
                        }
                    }
                ),
                file!(),
                line!(),
            ));
        }

        // Receive encrypted packet.
        let mut encrypted_packet = vec![0u8; _next_packet_size as usize];
        match UserService::read_from_socket(&mut self.socket, &mut encrypted_packet) {
            IoResult::Fin => {
                *is_fin = true;
                return Err(AppError::new(
                    &format!(
                        "unexpected FIN received (socket: {})",
                        match self.socket_addr {
                            Some(addr) => {
                                addr.to_string()
                            }
                            None => {
                                String::new()
                            }
                        }
                    ),
                    file!(),
                    line!(),
                ));
            }
            IoResult::Err(err) => return Err(err.add_entry(file!(), line!())),
            IoResult::Ok(_) => {}
        };

        // Get IV.
        if encrypted_packet.len() < IV_LENGTH {
            return Err(AppError::new(
                &format!(
                    "unexpected packet length ({}) (socket: {})",
                    encrypted_packet.len(),
                    match self.socket_addr {
                        Some(addr) => {
                            addr.to_string()
                        }
                        None => {
                            String::new()
                        }
                    }
                ),
                file!(),
                line!(),
            ));
        }
        let iv = &encrypted_packet[..IV_LENGTH].to_vec();
        encrypted_packet = encrypted_packet[IV_LENGTH..].to_vec();

        // Decrypt packet.
        let cipher = Aes256Cbc::new_from_slices(&self.secret_key, &iv).unwrap();
        let decrypted_packet = cipher.decrypt_vec(&encrypted_packet);
        if let Err(e) = decrypted_packet {
            return Err(AppError::new(
                &format!(
                    "{:?} (socket: {})",
                    e,
                    match self.socket_addr {
                        Some(addr) => {
                            addr.to_string()
                        }
                        None => {
                            String::new()
                        }
                    }
                ),
                file!(),
                line!(),
            ));
        }
        let mut decrypted_packet = decrypted_packet.unwrap();

        // CMAC
        let mut mac = Cmac::<Aes256>::new_from_slice(&self.secret_key).unwrap();
        let tag: Vec<u8> = decrypted_packet
            .drain(decrypted_packet.len().saturating_sub(CMAC_TAG_LENGTH)..)
            .collect();
        mac.update(&decrypted_packet);
        if let Err(e) = mac.verify(&tag) {
            return Err(AppError::new(
                &format!(
                    "{:?} (socket: {})",
                    e,
                    match self.socket_addr {
                        Some(addr) => {
                            addr.to_string()
                        }
                        None => {
                            String::new()
                        }
                    }
                ),
                file!(),
                line!(),
            ));
        }

        Ok(decrypted_packet)
    }
    /// Reads data from the specified socket.
    ///
    /// Arguments:
    /// - `socket`: socket to read the data from.
    /// - `buf`: buffer to write read data.
    fn read_from_socket(socket: &mut TcpStream, buf: &mut [u8]) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
        }

        let mut total_wait_time_ms: u64 = 0;

        loop {
            if total_wait_time_ms >= MAX_WAIT_TIME_IN_READ_WRITE_MS {
                return IoResult::Err(AppError::new(
                    &format!(
                        "reached maximum response wait time limit of {} ms for socket {}",
                        MAX_WAIT_TIME_IN_READ_WRITE_MS,
                        match socket.peer_addr() {
                            Ok(addr) => {
                                addr.to_string()
                            }
                            Err(_) => {
                                String::new()
                            }
                        },
                    ),
                    file!(),
                    line!(),
                ));
            }

            match socket.read(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(AppError::new(
                            &format!(
                                "failed to read (got: {}, expected: {}) (socket {})",
                                n,
                                buf.len(),
                                match socket.peer_addr() {
                                    Ok(addr) => {
                                        addr.to_string()
                                    }
                                    Err(_) => {
                                        String::new()
                                    }
                                },
                            ),
                            file!(),
                            line!(),
                        ));
                    }

                    return IoResult::Ok(n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                    continue;
                }
                Err(e) => {
                    return IoResult::Err(AppError::new(
                        &format!(
                            "{:?} (socket {})",
                            e,
                            match socket.peer_addr() {
                                Ok(addr) => {
                                    addr.to_string()
                                }
                                Err(_) => {
                                    String::new()
                                }
                            },
                        ),
                        file!(),
                        line!(),
                    ));
                }
            };
        }
    }
    /// Reads data from the specified socket.
    ///
    /// Arguments:
    /// - `socket`: socket to read the data from.
    /// - `buf`: buffer to write read data.
    ///
    /// Returns `None` if timeout reached.
    fn read_from_socket_with_timeout(
        socket: &mut TcpStream,
        buf: &mut [u8],
        timeout_in_ms: u64,
    ) -> Option<IoResult> {
        if buf.is_empty() {
            return Some(IoResult::Err(AppError::new(
                "passed 'buf' has 0 length",
                file!(),
                line!(),
            )));
        }

        let mut total_wait_time_ms: u64 = 0;

        loop {
            if total_wait_time_ms >= timeout_in_ms {
                return None;
            }

            match socket.read(buf) {
                Ok(0) => {
                    return Some(IoResult::Fin);
                }
                Ok(n) => {
                    if n != buf.len() {
                        return Some(IoResult::Err(AppError::new(
                            &format!(
                                "failed to read (got: {}, expected: {}) (socket {})",
                                n,
                                buf.len(),
                                match socket.peer_addr() {
                                    Ok(addr) => {
                                        addr.to_string()
                                    }
                                    Err(_) => {
                                        String::new()
                                    }
                                },
                            ),
                            file!(),
                            line!(),
                        )));
                    }

                    return Some(IoResult::Ok(n));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                    continue;
                }
                Err(e) => {
                    return Some(IoResult::Err(AppError::new(
                        &format!(
                            "{:?} (socket {})",
                            e,
                            match socket.peer_addr() {
                                Ok(addr) => {
                                    addr.to_string()
                                }
                                Err(_) => {
                                    String::new()
                                }
                            },
                        ),
                        file!(),
                        line!(),
                    )));
                }
            };
        }
    }
    /// Writes the specified buffer to the socket.
    ///
    /// Arguments:
    /// - `socket`: socket to write this data to.
    /// - `buf`: buffer to write to the socket.
    /// - `enable_wait_limit`: if `false` will wait for write operation to finish
    /// infinitely, otherwise will wait for maximum `MAX_WAIT_TIME_IN_READ_WRITE_MS`
    /// for operation to finish and return error in case of a timeout.
    fn write_to_socket(
        socket: &mut TcpStream,
        buf: &mut [u8],
        enable_wait_limit: bool,
    ) -> IoResult {
        if buf.is_empty() {
            return IoResult::Err(AppError::new("passed 'buf' has 0 length", file!(), line!()));
        }

        let mut total_wait_time_ms: u64 = 0;

        loop {
            if enable_wait_limit && total_wait_time_ms >= MAX_WAIT_TIME_IN_READ_WRITE_MS {
                return IoResult::Err(AppError::new(
                    &format!(
                        "reached maximum response wait time limit of {} ms for socket {}",
                        MAX_WAIT_TIME_IN_READ_WRITE_MS,
                        match socket.peer_addr() {
                            Ok(addr) => {
                                addr.to_string()
                            }
                            Err(_) => {
                                String::new()
                            }
                        },
                    ),
                    file!(),
                    line!(),
                ));
            }

            match socket.write(buf) {
                Ok(0) => {
                    return IoResult::Fin;
                }
                Ok(n) => {
                    if n != buf.len() {
                        return IoResult::Err(AppError::new(
                            &format!(
                                "failed to write (got: {}, expected: {}) (socket {})",
                                n,
                                buf.len(),
                                match socket.peer_addr() {
                                    Ok(addr) => {
                                        addr.to_string()
                                    }
                                    Err(_) => {
                                        String::new()
                                    }
                                },
                            ),
                            file!(),
                            line!(),
                        ));
                    }

                    return IoResult::Ok(n);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(WOULD_BLOCK_RETRY_AFTER_MS));
                    total_wait_time_ms += WOULD_BLOCK_RETRY_AFTER_MS;
                    continue;
                }
                Err(e) => {
                    return IoResult::Err(AppError::new(
                        &format!(
                            "{:?} (socket {})",
                            e,
                            match socket.peer_addr() {
                                Ok(addr) => {
                                    addr.to_string()
                                }
                                Err(_) => {
                                    String::new()
                                }
                            }
                        ),
                        file!(),
                        line!(),
                    ));
                }
            };
        }
    }
}

impl Drop for UserService {
    /// Logs information about connection being closed.
    fn drop(&mut self) {
        let mut _message = String::new();

        if self.username.is_some() {
            _message = format!("{} logged out", self.username.as_ref().unwrap());
        } else {
            _message = format!(
                "Closing connection with {}",
                match self.socket_addr {
                    Some(addr) => {
                        addr.to_string()
                    }
                    None => {
                        String::new()
                    }
                }
            );
        }

        if self.exit_error.is_some() {
            let error = self.exit_error.as_ref().unwrap();

            if let Err(app_error) = error {
                _message += &format!(" due to internal error (bug):\n{}", app_error);
            } else {
                _message += &format!(", reason: {}", error.as_ref().unwrap());
            }
        }

        if !_message.ends_with('.') {
            _message += ".";
        }

        _message += "\n";

        let mut guard = self.connected_users_count.lock().unwrap();
        *guard -= 1;
        _message += &format!("--- [connected: {}] ---", guard);

        self.logger
            .lock()
            .unwrap()
            .print_and_log(LogCategory::Info, &_message);
    }
}
