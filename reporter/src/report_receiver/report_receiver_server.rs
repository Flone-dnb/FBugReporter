// Std.
use std::net::*;
use std::time::Duration;

// Custom.
use super::*;
use crate::log_manager::LogManager;
use shared::misc::error::AppError;
use shared::network::messaging::*;
use shared::network::net_params::*;
use shared::network::reporter_messages::*;

pub struct ReportReceiverServer {}

impl ReportReceiver for ReportReceiverServer {
    fn request_max_attachment_size_in_mb(
        &mut self,
        remote_address: String,
        logger: &mut LogManager,
    ) -> Option<usize> {
        let result = Self::establish_secure_connection_with_server(remote_address, logger);

        // Check for errors.
        if let Err(app_error) = result {
            if let Some(err) = app_error {
                logger.log(&err.to_string());
                return None;
            } else {
                logger.log("Could not connect to the server.");
                return None;
            }
        }
        let (mut tcp_socket, secret_key) = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::MaxAttachmentSize {};

        // Send message.
        if let Some(app_error) = send_message(&mut tcp_socket, &secret_key, message) {
            logger.log(&app_error.to_string());
            return None;
        }

        let mut is_fin = false;
        let result = receive_message(
            &mut tcp_socket,
            &secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );

        // Check for errors.
        if let Err(app_error) = result {
            logger.log(&app_error.to_string());
            return None;
        }
        let result = result.unwrap();

        if is_fin {
            logger.log(&AppError::new("the server closed connection unexpectedly").to_string());
            return None;
        }

        // Deserialize.
        let received_message = bincode::deserialize::<ReporterAnswer>(&result);
        if let Err(e) = received_message {
            logger.log(&AppError::new(&e.to_string()).to_string());
            return None;
        }
        let received_message = received_message.unwrap();

        // Process answer.
        match received_message {
            ReporterAnswer::MaxAttachmentSize {
                max_attachments_size_in_mb,
            } => Some(max_attachments_size_in_mb),
            _ => {
                logger.log(
                    &AppError::new(&format!(
                        "received unexpected answer from the server ({:?})",
                        received_message
                    ))
                    .to_string(),
                );
                None
            }
        }
    }

    /// Sends the specified report to the specified remote address.
    ///
    /// ## Arguments
    /// * `remote_address` string in the form "IP:PORT" where the first part is server's
    /// IP address and the second one is server's port for reporters.
    /// * `auth_token` not used.
    /// * `report` report to send.
    /// * `logger` logger that will be used to write to logs.
    /// * `attachments` report attachements.
    fn send_report(
        &mut self,
        remote_address: String,
        _auth_token: String,
        report: GameReport,
        logger: &mut LogManager,
        attachments: Vec<ReportAttachment>,
    ) -> SendReportResult {
        let result = Self::establish_secure_connection_with_server(remote_address, logger);
        if let Err(app_error) = result {
            if let Some(err) = app_error {
                logger.log(&err.to_string());
                return SendReportResult::Other(err.get_message());
            } else {
                logger.log("Could not connect to the server.");
                return SendReportResult::CouldNotConnect;
            }
        }
        let (mut tcp_socket, secret_key) = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::Report {
            reporter_net_protocol: NETWORK_PROTOCOL_VERSION,
            game_report: Box::new(report),
            attachments,
        };

        logger.log("Sending report message to the server.");

        // Send message.
        if let Some(app_error) = send_message(&mut tcp_socket, &secret_key, message) {
            logger.log(&app_error.to_string());
            return SendReportResult::Other(app_error.get_message());
        }

        logger.log("Sent report message.");
        logger.log("Waiting for server to answer.");

        let mut is_fin = false;
        let result = receive_message(
            &mut tcp_socket,
            &secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            let app_error = AppError::new("the server closed connection unexpectedly");
            logger.log(&app_error.to_string());
            return SendReportResult::Other(app_error.get_message());
        }
        if let Err(app_error) = result {
            logger.log(&app_error.to_string());
            return SendReportResult::Other(app_error.get_message());
        }

        let result = result.unwrap();

        logger.log("Received an answer from the server.");

        // Deserialize.
        let received_message = bincode::deserialize::<ServerAnswer>(&result);
        if let Err(e) = received_message {
            let app_error = AppError::new(&e.to_string());
            logger.log(&app_error.to_string());
            return SendReportResult::Other(app_error.get_message());
        }
        let received_message = received_message.unwrap();

        // Process answer.
        match received_message {
            ServerAnswer::Ok => SendReportResult::Ok,
            ServerAnswer::OtherError(message) => SendReportResult::Other(message),
        }
    }
}

impl ReportReceiverServer {
    pub fn new() -> Self {
        Self {}
    }
    /// Connects to the server and establishes a secure connection.
    ///
    /// ## Return
    /// `Ok` with socket and established secret key if successful, otherwise
    /// `None` if unable to connect to the server and `Some` if internal error occurred.
    fn establish_secure_connection_with_server(
        server_addr: String,
        logger: &mut LogManager,
    ) -> Result<(TcpStream, [u8; SECRET_KEY_SIZE]), Option<AppError>> {
        let addrs = server_addr.to_socket_addrs();
        if let Err(e) = addrs {
            return Err(Some(AppError::new(&e.to_string())));
        }
        let addrs = addrs.unwrap();

        let mut tcp_socket: Option<TcpStream> = None;
        for addr in addrs {
            let result = TcpStream::connect_timeout(&addr, Duration::from_secs(2));
            if let Ok(socket) = result {
                tcp_socket = Some(socket);
                break;
            }
        }

        if tcp_socket.is_none() {
            return Err(None);
        }

        let mut tcp_socket = tcp_socket.unwrap();

        logger.log("Connected to the server.");

        if let Err(e) = tcp_socket.set_nodelay(true) {
            return Err(Some(AppError::new(&e.to_string())));
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return Err(Some(AppError::new(&e.to_string())));
        }

        let secret_key = accept_secure_connection_establishment(&mut tcp_socket);
        if let Err(app_error) = secret_key {
            return Err(Some(app_error));
        } else {
            logger.log("Secure connection established.");
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return Err(Some(AppError::new(
                "failed to convert Vec<u8> to generic array",
            )));
        }
        let secret_key: [u8; SECRET_KEY_SIZE] = result.unwrap();

        Ok((tcp_socket, secret_key))
    }
}
