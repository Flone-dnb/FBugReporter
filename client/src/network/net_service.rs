// Std.
use std::fs;
use std::net::*;
use std::path::Path;
use std::time::Duration;

// External.
use sha2::{Digest, Sha512};

// Custom.
use crate::io::config_manager::ConfigManager;
use shared::misc::db_manager::ReportData;
use shared::misc::error::AppError;
use shared::misc::report::ReportSummary;
use shared::network::client_messages::*;
use shared::network::messaging::*;
use shared::network::net_params::*;

pub enum ConnectResult {
    Connected(bool),
    ConnectFailed(String),
    NeedFirstPassword,
    SetupOTP(String),
    NeedOTP,
    InternalError(AppError),
}

pub struct NetService {
    socket: Option<TcpStream>,
    secret_key: [u8; SECRET_KEY_SIZE],
    is_connected: bool,
}

impl NetService {
    pub fn new() -> Self {
        Self {
            socket: None,
            secret_key: [0; SECRET_KEY_SIZE],
            is_connected: false,
        }
    }
    /// Tries to connect to the server.
    ///
    /// OTP string might be empty if the user still have not received the OTP QR code.
    /// Once everything is correct, the server will see empty OTP string and if OTP
    /// is enabled for this user (default) the server will respond with OTP QR code
    /// that we can show to the user and connect again with a valid OTP.
    ///
    /// Specify a `new_password` if you want to send the first password (changed password).
    pub fn connect(
        &mut self,
        server: String,
        port: u16,
        username: String,
        password: String,
        otp: String,
        new_password: Option<String>,
    ) -> ConnectResult {
        let addrs = format!("{}:{}", server, port).to_socket_addrs();
        if let Err(e) = addrs {
            return ConnectResult::InternalError(AppError::new(&e.to_string()));
        }
        let addrs = addrs.unwrap();

        let mut tcp_socket: Option<TcpStream> = None;
        for addr in addrs {
            let result = TcpStream::connect_timeout(&addr, Duration::from_secs(2));
            if result.is_ok() {
                tcp_socket = Some(result.unwrap());
                break;
            }
        }

        if tcp_socket.is_none() {
            return ConnectResult::InternalError(AppError::new(
                "could not connect to the \
                        server, make sure that server name and port are correct and the \
                        server is running",
            ));
        }

        let tcp_socket = tcp_socket.unwrap();

        // Configure socket.
        if let Err(e) = tcp_socket.set_nodelay(true) {
            return ConnectResult::InternalError(AppError::new(&e.to_string()));
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return ConnectResult::InternalError(AppError::new(&e.to_string()));
        }

        self.socket = Some(tcp_socket);

        // Establish secure connection.
        let secret_key = accept_secure_connection_establishment(self.socket.as_mut().unwrap());
        if let Err(app_error) = secret_key {
            return ConnectResult::InternalError(app_error);
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return ConnectResult::InternalError(AppError::new(
                "failed to convert Vec<u8> to generic array",
            ));
        }
        self.secret_key = result.unwrap();

        // Generate password hash.
        let mut hasher = Sha512::new();
        hasher.update(password.as_bytes());
        let password = hasher.finalize().to_vec();

        // Prepare packet to send.
        let mut packet = ClientRequest::Login {
            client_net_protocol: NETWORK_PROTOCOL_VERSION,
            username: username.clone(),
            password: password.clone(),
            otp,
        };

        if new_password.is_some() {
            // Generate new password hash.
            hasher = Sha512::new();
            hasher.update(new_password.unwrap().as_bytes());
            let new_password = hasher.finalize().to_vec();

            // Update packet to send.
            packet = ClientRequest::SetFirstPassword {
                client_net_protocol: NETWORK_PROTOCOL_VERSION,
                username: username.clone(),
                old_password: password,
                new_password,
            }
        }

        if let Some(app_error) =
            send_message(self.socket.as_mut().unwrap(), &self.secret_key, packet)
        {
            return ConnectResult::InternalError(app_error);
        }

        // Receive answer.
        let mut is_fin = false;
        let packet = receive_message(
            self.socket.as_mut().unwrap(),
            &self.secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return ConnectResult::InternalError(AppError::new("unexpected FIN received"));
        }
        if let Err(app_error) = packet {
            return ConnectResult::InternalError(app_error);
        }
        let packet = packet.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<ClientAnswer>(&packet);
        if let Err(e) = packet {
            return ConnectResult::InternalError(AppError::new(&e.to_string()));
        }
        let packet = packet.unwrap();

        let mut _is_admin = false;
        match packet {
            ClientAnswer::LoginAnswer {
                is_ok,
                is_admin,
                fail_reason,
            } => {
                if !is_ok {
                    let mut _message = String::new();
                    match fail_reason.unwrap() {
                        ClientLoginFailReason::WrongProtocol { server_protocol } => {
                            _message = format!(
                                "Failed to connect to the server \
                            due to incompatible application version.\n\
                            Your application uses network protocol version {}, \
                            while the server supports version {}.",
                                NETWORK_PROTOCOL_VERSION, server_protocol
                            );
                        }
                        ClientLoginFailReason::WrongCredentials { result } => match result {
                            ClientLoginFailResult::FailedAttempt {
                                failed_attempts_made,
                                max_failed_attempts,
                            } => {
                                _message = format!(
                                    "Incorrect login/password/OTP.\n\
                                Allowed failed login attempts: {0} out of {1}.\n\
                                After {1} failed login attempts new failed login attempt \
                                 will result in a ban.",
                                    failed_attempts_made, max_failed_attempts
                                );
                            }
                            ClientLoginFailResult::Banned { ban_time_in_min } => {
                                _message = format!(
                                    "You were banned due to multiple failed login attempts.\n\
                                Ban time: {} minute(-s).\n\
                                During this time the server will reject any \
                                login attempts without explanation.",
                                    ban_time_in_min
                                );
                            }
                        },
                        ClientLoginFailReason::SetupOTP { qr_code } => {
                            return ConnectResult::SetupOTP(qr_code);
                        }
                        ClientLoginFailReason::NeedOTP => return ConnectResult::NeedOTP,
                        ClientLoginFailReason::NeedFirstPassword => {
                            return ConnectResult::NeedFirstPassword;
                        }
                    }
                    return ConnectResult::ConnectFailed(_message);
                } else {
                    _is_admin = is_admin;
                }
            }
            _ => {
                return ConnectResult::InternalError(AppError::new("unexpected packet received"));
            }
        }

        // Connected.
        let mut config = ConfigManager::new();
        config.server = server;
        config.port = port.to_string();
        config.username = username;
        config.write_config_to_file();

        self.is_connected = true;

        // Return control here, don't drop the connection,
        // wait for further commands from the user.
        ConnectResult::Connected(_is_admin)
    }
    pub fn query_reports(
        &mut self,
        page: u64,
        amount: u64,
    ) -> Result<(Vec<ReportSummary>, u64), AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected"));
        }

        // Prepare packet to send.
        let packet = ClientRequest::QueryReportsSummary { page, amount };

        if let Some(app_error) =
            send_message(self.socket.as_mut().unwrap(), &self.secret_key, packet)
        {
            return Err(app_error);
        }

        let mut is_fin = false;
        let result = receive_message(
            self.socket.as_mut().unwrap(),
            &self.secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return Err(AppError::new("unexpected FIN received"));
        }
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string()));
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<ClientAnswer>(&serialized_packet);
        if let Err(e) = packet {
            return Err(AppError::new(&e.to_string()));
        }
        let packet = packet.unwrap();

        match packet {
            ClientAnswer::ReportsSummary {
                reports,
                total_reports,
            } => {
                return Ok((reports, total_reports));
            }
            _ => {
                return Err(AppError::new("unexpected packet received"));
            }
        }
    }

    /// Downloads and saves an attachment from the server.
    ///
    /// ## Return
    /// `Err(AppError)` if something went wrong, otherwise
    /// `Ok(true)` if attachment is found and saved,
    /// `Ok(false)` if attachment is not found.
    pub fn download_attachment(
        &mut self,
        attachment_id: usize,
        path_to_save: &Path,
    ) -> Result<bool, AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected"));
        }

        // Prepare packet to send.
        let message = ClientRequest::QueryAttachment { attachment_id };

        // Send message.
        if let Some(app_error) =
            send_message(self.socket.as_mut().unwrap(), &self.secret_key, message)
        {
            return Err(app_error);
        }

        // Wait for answer.
        let mut is_fin = false;
        let result = receive_message(
            self.socket.as_mut().unwrap(),
            &self.secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return Err(AppError::new("unexpected FIN received"));
        }
        if let Err(app_error) = result {
            return Err(app_error);
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let message = bincode::deserialize::<ClientAnswer>(&serialized_packet);
        if let Err(e) = message {
            return Err(AppError::new(&e.to_string()));
        }
        let message = message.unwrap();

        match message {
            ClientAnswer::Attachment { is_found, data } => {
                if !is_found {
                    return Ok(false);
                } else {
                    if let Err(e) = fs::write(path_to_save, data) {
                        return Err(AppError::new(&e.to_string()));
                    }
                    return Ok(true);
                }
            }
            _ => {
                return Err(AppError::new("unexpected message received"));
            }
        }
    }
    pub fn query_report(&mut self, report_id: u64) -> Result<ReportData, AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected"));
        }

        // Prepare packet to send.
        let message = ClientRequest::QueryReport { report_id };

        // Send message.
        if let Some(app_error) =
            send_message(self.socket.as_mut().unwrap(), &self.secret_key, message)
        {
            return Err(app_error);
        }

        // Wait for answer.
        let mut is_fin = false;
        let result = receive_message(
            self.socket.as_mut().unwrap(),
            &self.secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return Err(AppError::new("unexpected FIN received"));
        }
        if let Err(app_error) = result {
            return Err(app_error);
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let message = bincode::deserialize::<ClientAnswer>(&serialized_packet);
        if let Err(e) = message {
            return Err(AppError::new(&e.to_string()));
        }
        let message = message.unwrap();

        match message {
            ClientAnswer::Report {
                id,
                title,
                game_name,
                game_version,
                text,
                date,
                time,
                sender_name,
                sender_email,
                os_info,
                attachments,
            } => {
                return Ok(ReportData {
                    id,
                    title,
                    game_name,
                    game_version,
                    text,
                    date,
                    time,
                    sender_name,
                    sender_email,
                    os_info,
                    attachments,
                });
            }
            _ => {
                return Err(AppError::new("unexpected message received"));
            }
        }
    }
    pub fn delete_report(&mut self, report_id: u64) -> Result<bool, AppError> {
        if !self.is_connected {
            return Err(AppError::new("not connected"));
        }

        // Prepare packet to send.
        let packet = ClientRequest::DeleteReport { report_id };

        if let Some(app_error) =
            send_message(self.socket.as_mut().unwrap(), &self.secret_key, packet)
        {
            return Err(app_error);
        }

        let mut is_fin = false;
        let result = receive_message(
            self.socket.as_mut().unwrap(),
            &self.secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return Err(AppError::new("unexpected FIN received"));
        }
        if let Err(e) = result {
            return Err(AppError::new(&e.to_string()));
        }
        let serialized_packet = result.unwrap();

        // Deserialize.
        let packet = bincode::deserialize::<ClientAnswer>(&serialized_packet);
        if let Err(e) = packet {
            return Err(AppError::new(&e.to_string()));
        }
        let packet = packet.unwrap();

        match packet {
            ClientAnswer::DeleteReportResult {
                is_found_and_removed,
            } => {
                return Ok(is_found_and_removed);
            }
            _ => {
                return Err(AppError::new("unexpected packet received"));
            }
        }
    }
}
