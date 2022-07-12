// Std.
use std::net::*;

// Custom.
use crate::log_manager::LogManager;
use shared::misc::error::AppError;
use shared::misc::report::*;
use shared::network::messaging::*;
use shared::network::net_params::*;
use shared::network::reporter_packets::*;

pub struct ReporterService {
    tcp_socket: Option<TcpStream>,
}

impl ReporterService {
    pub fn new() -> Self {
        Self { tcp_socket: None }
    }
    pub fn send_report(
        &mut self,
        server_addr: SocketAddrV4,
        report: GameReport,
        logger: &mut LogManager,
        attachments: Vec<ReportAttachment>,
    ) -> (ReportResult, Option<AppError>) {
        let tcp_socket = TcpStream::connect(server_addr);

        if let Err(e) = tcp_socket {
            return (
                ReportResult::CouldNotConnect,
                Some(AppError::new(&e.to_string(), file!(), line!())),
            );
        } else {
            logger.log("Connected to the server.");
        }

        let tcp_socket = tcp_socket.unwrap();
        if let Err(e) = tcp_socket.set_nodelay(true) {
            return (
                ReportResult::InternalError,
                Some(AppError::new(&e.to_string(), file!(), line!())),
            );
        }
        if let Err(e) = tcp_socket.set_nonblocking(true) {
            return (
                ReportResult::InternalError,
                Some(AppError::new(&e.to_string(), file!(), line!())),
            );
        }
        self.tcp_socket = Some(tcp_socket);

        let secret_key = accept_secure_connection_establishment(self.tcp_socket.as_mut().unwrap());
        if let Err(app_error) = secret_key {
            return (
                ReportResult::InternalError,
                Some(app_error.add_entry(file!(), line!())),
            );
        } else {
            logger.log("Secure connection established.");
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            return (
                ReportResult::InternalError,
                Some(AppError::new(
                    "failed to convert Vec<u8> to generic array",
                    file!(),
                    line!(),
                )),
            );
        }
        let secret_key: [u8; SECRET_KEY_SIZE] = result.unwrap();

        // Prepare message.
        let message = ReporterRequest::Report {
            reporter_net_protocol: NETWORK_PROTOCOL_VERSION,
            game_report: report,
            attachments,
        };

        // Send message.
        if let Some(app_error) =
            send_message(self.tcp_socket.as_mut().unwrap(), &secret_key, message)
        {
            return (
                ReportResult::InternalError,
                Some(app_error.add_entry(file!(), line!())),
            );
        }

        let mut is_fin = false;
        let result = receive_message(
            self.tcp_socket.as_mut().unwrap(),
            &secret_key,
            None,
            std::u64::MAX,
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
            ReporterAnswer::ReportRequestResult { result_code } => {
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
        }
    }
}
