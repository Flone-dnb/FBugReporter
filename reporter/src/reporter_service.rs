// Std.
use std::net::*;
use std::time::Duration;

// Custom.
use crate::log_manager::LogManager;
use shared::misc::error::AppError;
use shared::misc::report::*;
use shared::network::messaging::*;
use shared::network::net_params::*;
use shared::network::reporter_messages::*;

pub struct ReporterService {}

impl ReporterService {
    pub fn new() -> Self {
        Self {}
    }

    /// Requests maximum allowed size of attachments (in total) in MB.
    ///
    /// ## Arguments
    /// * `server_addr`: address of the server to connect to.
    /// * `logger`: logger to use.
    ///
    /// ## Return
    /// An error if something went wrong, otherwise maximum allowed size of attachments.
    pub fn request_max_attachment_size_in_mb(
        &mut self,
        server_addr: String,
        logger: &mut LogManager,
    ) -> Result<usize, AppError> {
        let result = Self::establish_secure_connection_with_server(server_addr, logger);
        if let Err(app_error) = result {
            return Err(app_error);
        }
        let (mut tcp_socket, secret_key) = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::MaxAttachmentSize {};

        // Send message.
        if let Some(app_error) = send_message(&mut tcp_socket, &secret_key, message) {
            return Err(app_error);
        }

        let mut is_fin = false;
        let result = receive_message(
            &mut tcp_socket,
            &secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return Err(AppError::new("the server closed connection unexpectedly"));
        }
        if let Err(app_error) = result {
            return Err(app_error);
        }

        let result = result.unwrap();

        // Deserialize.
        let received_message = bincode::deserialize::<ReporterAnswer>(&result);
        if let Err(e) = received_message {
            return Err(AppError::new(&e.to_string()));
        }
        let received_message = received_message.unwrap();

        // Process answer.
        match received_message {
            ReporterAnswer::MaxAttachmentSize {
                max_attachments_size_in_mb,
            } => Ok(max_attachments_size_in_mb),
            _ => Err(AppError::new(&format!(
                "received unexpected answer from the server ({:?})",
                received_message
            ))),
        }
    }

    pub fn send_report(
        &mut self,
        server_addr: String,
        report: GameReport,
        logger: &mut LogManager,
        attachments: Vec<ReportAttachment>,
    ) -> (ReportResult, Option<AppError>) {
        let result = Self::establish_secure_connection_with_server(server_addr, logger);
        if let Err(app_error) = result {
            return (ReportResult::InternalError, Some(app_error));
        }
        let (mut tcp_socket, secret_key) = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::Report {
            reporter_net_protocol: NETWORK_PROTOCOL_VERSION,
            game_report: Box::new(report),
            attachments,
        };

        // Send message.
        if let Some(app_error) = send_message(&mut tcp_socket, &secret_key, message) {
            return (ReportResult::InternalError, Some(app_error));
        }

        let mut is_fin = false;
        let result = receive_message(
            &mut tcp_socket,
            &secret_key,
            None,
            std::usize::MAX,
            &mut is_fin,
        );
        if is_fin {
            return (
                ReportResult::InternalError,
                Some(AppError::new("the server closed connection unexpectedly")),
            );
        }
        if let Err(app_error) = result {
            return (ReportResult::InternalError, Some(app_error));
        }

        let result = result.unwrap();

        // Deserialize.
        let received_message = bincode::deserialize::<ReporterAnswer>(&result);
        if let Err(e) = received_message {
            return (
                ReportResult::InternalError,
                Some(AppError::new(&e.to_string())),
            );
        }
        let received_message = received_message.unwrap();

        // Process answer.
        match received_message {
            ReporterAnswer::Report { result_code } => {
                if result_code != ReportResult::Ok {
                    return (
                        result_code,
                        Some(AppError::new(&format!(
                            "The server returned error: {:?}",
                            result_code
                        ))),
                    );
                }
                (result_code, None)
            }
            _ => (
                ReportResult::InternalError,
                Some(AppError::new(&format!(
                    "received unexpected answer from the server ({:?})",
                    received_message
                ))),
            ),
        }
    }

    /// Connects to the server and establishes a secure connection.
    fn establish_secure_connection_with_server(
        server_addr: String,
        logger: &mut LogManager,
    ) -> Result<(TcpStream, [u8; SECRET_KEY_SIZE]), AppError> {
        let addrs = server_addr.to_socket_addrs();
        if let Err(e) = addrs {
            return Err(AppError::new(&e.to_string()));
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
            return Err(AppError::new("the server is offline, try again later"));
        }

        let mut tcp_socket = tcp_socket.unwrap();

        logger.log("Connected to the server.");

        if let Err(e) = tcp_socket.set_nodelay(true) {
            return Err(AppError::new(&e.to_string()));
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return Err(AppError::new(&e.to_string()));
        }

        let secret_key = accept_secure_connection_establishment(&mut tcp_socket);
        if let Err(app_error) = secret_key {
            return Err(app_error);
        } else {
            logger.log("Secure connection established.");
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return Err(AppError::new("failed to convert Vec<u8> to generic array"));
        }
        let secret_key: [u8; SECRET_KEY_SIZE] = result.unwrap();

        Ok((tcp_socket, secret_key))
    }
}
