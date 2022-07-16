// Std.
use std::net::*;

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
        server_addr: SocketAddrV4,
        logger: &mut LogManager,
    ) -> Result<usize, AppError> {
        let result = Self::establish_secure_connection_with_server(server_addr, logger);
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }
        let (mut tcp_socket, secret_key) = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::MaxAttachmentSize {};

        // Send message.
        if let Some(app_error) = send_message(&mut tcp_socket, &secret_key, message) {
            return Err(app_error.add_entry(file!(), line!()));
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
            return Err(AppError::new(
                "the server closed connection unexpectedly",
                file!(),
                line!(),
            ));
        }
        if let Err(app_error) = result {
            return Err(app_error.add_entry(file!(), line!()));
        }

        let result = result.unwrap();

        // Deserialize.
        let received_message = bincode::deserialize::<ReporterAnswer>(&result);
        if let Err(e) = received_message {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        let received_message = received_message.unwrap();

        // Process answer.
        match received_message {
            ReporterAnswer::MaxAttachmentSize {
                max_attachments_size_in_mb,
            } => {
                return Ok(max_attachments_size_in_mb);
            }
            _ => {
                return Err(AppError::new(
                    &format!(
                        "received unexpected answer from the server ({:?})",
                        received_message
                    ),
                    file!(),
                    line!(),
                ));
            }
        }
    }

    pub fn send_report(
        &mut self,
        server_addr: SocketAddrV4,
        report: GameReport,
        logger: &mut LogManager,
        attachments: Vec<ReportAttachment>,
    ) -> (ReportResult, Option<AppError>) {
        let result = Self::establish_secure_connection_with_server(server_addr, logger);
        if let Err(app_error) = result {
            return (
                ReportResult::InternalError,
                Some(app_error.add_entry(file!(), line!())),
            );
        }
        let (mut tcp_socket, secret_key) = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::Report {
            reporter_net_protocol: NETWORK_PROTOCOL_VERSION,
            game_report: report,
            attachments,
        };

        // Send message.
        if let Some(app_error) = send_message(&mut tcp_socket, &secret_key, message) {
            return (
                ReportResult::InternalError,
                Some(app_error.add_entry(file!(), line!())),
            );
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
                Some(AppError::new(
                    "the server closed connection unexpectedly",
                    file!(),
                    line!(),
                )),
            );
        }
        if let Err(app_error) = result {
            return (
                ReportResult::InternalError,
                Some(app_error.add_entry(file!(), line!())),
            );
        }

        let result = result.unwrap();

        // Deserialize.
        let received_message = bincode::deserialize::<ReporterAnswer>(&result);
        if let Err(e) = received_message {
            return (
                ReportResult::InternalError,
                Some(AppError::new(&e.to_string(), file!(), line!())),
            );
        }
        let received_message = received_message.unwrap();

        // Process answer.
        match received_message {
            ReporterAnswer::Report { result_code } => {
                if result_code != ReportResult::Ok {
                    return (
                        result_code,
                        Some(AppError::new(
                            &format!("The server returned error: {:?}", result_code),
                            file!(),
                            line!(),
                        )),
                    );
                }
                return (result_code, None);
            }
            _ => {
                return (
                    ReportResult::InternalError,
                    Some(AppError::new(
                        &format!(
                            "received unexpected answer from the server ({:?})",
                            received_message
                        ),
                        file!(),
                        line!(),
                    )),
                );
            }
        }
    }

    /// Connects to the server and establishes a secure connection.
    fn establish_secure_connection_with_server(
        server_addr: SocketAddrV4,
        logger: &mut LogManager,
    ) -> Result<(TcpStream, [u8; SECRET_KEY_SIZE]), AppError> {
        let tcp_socket = TcpStream::connect(server_addr);

        if let Err(e) = tcp_socket {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        } else {
            logger.log("Connected to the server.");
        }

        let mut tcp_socket = tcp_socket.unwrap();
        if let Err(e) = tcp_socket.set_nodelay(true) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        let secret_key = accept_secure_connection_establishment(&mut tcp_socket);
        if let Err(app_error) = secret_key {
            return Err(app_error.add_entry(file!(), line!()));
        } else {
            logger.log("Secure connection established.");
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return Err(AppError::new(
                "failed to convert Vec<u8> to generic array",
                file!(),
                line!(),
            ));
        }
        let secret_key: [u8; SECRET_KEY_SIZE] = result.unwrap();

        Ok((tcp_socket, secret_key))
    }
}
