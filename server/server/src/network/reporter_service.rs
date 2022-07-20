// Std.
use std::net::*;
use std::sync::{Arc, Mutex};

// Custom.
use super::net_service::MAX_MESSAGE_SIZE_IN_BYTES_WITHOUT_ATTACHMENTS;
use crate::io::log_manager::*;
use shared::misc::db_manager::DatabaseManager;
use shared::misc::error::AppError;
use shared::misc::report::*;
use shared::network::messaging::*;
use shared::network::net_params::*;
use shared::network::reporter_messages::*;

pub struct ReporterService {
    logger: Arc<Mutex<LogManager>>,
    database: Arc<Mutex<DatabaseManager>>,
    socket: TcpStream,
    socket_addr: SocketAddr,
    secret_key: [u8; SECRET_KEY_SIZE],
    connected_count: Arc<Mutex<usize>>,
    exit_error: Option<Result<String, AppError>>,
    max_total_attachment_size_in_mb: usize,
}

impl ReporterService {
    /// Creates a new reporter service to process reporter requests.
    ///
    /// ## Arguments
    /// * `logger`: log manager for logging.
    /// * `socket`: connected reporter socket.
    /// * `addr`: reporter socket address.
    /// * `connected_users_count`: shared variable that stores total connections.
    /// * `database`: database manager that handles the database.
    /// * `max_attachment_size_in_mb`: maximum size of report attachments (in total) in MB.
    pub fn new(
        logger: Arc<Mutex<LogManager>>,
        socket: TcpStream,
        addr: SocketAddr,
        connected_count: Arc<Mutex<usize>>,
        database: Arc<Mutex<DatabaseManager>>,
        max_attachment_size_in_mb: usize,
    ) -> Self {
        {
            let mut guard = connected_count.lock().unwrap();
            *guard += 1;
            logger.lock().unwrap().print_and_log(
                LogCategory::Info,
                &format!(
                    "accepted connection with reporter {}:{}\n------------------------- \
                    [connected: {}] -------------------------",
                    addr.ip(),
                    addr.port(),
                    guard
                ),
            );
        }

        let socket_addr = socket.peer_addr().unwrap();

        Self {
            logger,
            socket,
            connected_count,
            exit_error: None,
            secret_key: [0; SECRET_KEY_SIZE],
            database,
            socket_addr,
            max_total_attachment_size_in_mb: max_attachment_size_in_mb,
        }
    }

    /// Processes reporter requests until finished communication.
    ///
    /// After this function is finished the object should be destroyed.
    pub fn process(mut self) {
        let secret_key = start_establishing_secure_connection(&mut self.socket);
        if let Err(app_error) = secret_key {
            self.exit_error = Some(Err(app_error));
            return;
        }
        let result = secret_key.unwrap().try_into();
        if result.is_err() {
            self.exit_error = Some(Err(AppError::new(
                "failed to convert Vec<u8> to generic array",
            )));
            return;
        }
        self.secret_key = result.unwrap();

        let max_allowed_message_size = MAX_MESSAGE_SIZE_IN_BYTES_WITHOUT_ATTACHMENTS
            + (self.max_total_attachment_size_in_mb * 1024 * 1024);

        // Wait for message.
        let mut is_fin = false; // don't check, react to FIN as error
        let message = receive_message(
            &mut self.socket,
            &self.secret_key,
            Some(MAX_WAIT_TIME_IN_READ_WRITE_MS),
            max_allowed_message_size,
            &mut is_fin,
        );
        if let Err(app_error) = message {
            self.exit_error = Some(Err(app_error));
            return;
        }
        let message = message.unwrap();

        // Deserialize.
        let message = bincode::deserialize::<ReporterRequest>(&message);
        if let Err(e) = message {
            self.exit_error = Some(Err(AppError::new(&e.to_string())));
            return;
        }
        let message = message.unwrap();

        let result = self.handle_reporter_message(message);
        if let Err(app_error) = result {
            self.exit_error = Some(Err(app_error));
            return;
        }
        let result = result.unwrap();
        if let Some(soft_error) = result {
            self.exit_error = Some(Ok(soft_error));
            //return;
        }
    }

    /// Processes the client message.
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
    fn handle_reporter_message(
        &mut self,
        message: ReporterRequest,
    ) -> Result<Option<String>, AppError> {
        match message {
            ReporterRequest::Report {
                reporter_net_protocol,
                game_report,
                attachments,
            } => self.handle_report_request(reporter_net_protocol, game_report, attachments),
            ReporterRequest::MaxAttachmentSize {} => {
                let result = self.handle_attachment_size_query_request();
                if let Some(app_error) = result {
                    return Err(app_error);
                }
                Ok(None)
            }
        }
    }

    /// Processes reporter's report request.
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
    fn handle_report_request(
        &mut self,
        reporter_net_protocol: u16,
        game_report: GameReport,
        attachments: Vec<ReportAttachment>,
    ) -> Result<Option<String>, AppError> {
        // Check protocol version.
        if reporter_net_protocol != NETWORK_PROTOCOL_VERSION {
            let result_code = ReportResult::WrongProtocol;

            // Notify reporter.
            if let Some(app_error) = send_message(
                &mut self.socket,
                &self.secret_key,
                ReporterAnswer::Report { result_code },
            ) {
                return Err(app_error);
            }

            return Ok(Some(format!(
                "wrong protocol version (reporter: {}, our: {})",
                reporter_net_protocol, NETWORK_PROTOCOL_VERSION
            )));
        }

        // Check field limits.
        if let Err((field, length)) = Self::check_report_field_limits(&game_report) {
            let result_code = ReportResult::ServerRejected;

            // Notify reporter.
            if let Some(app_error) = send_message(
                &mut self.socket,
                &self.secret_key,
                ReporterAnswer::Report { result_code },
            ) {
                return Err(app_error);
            }

            return Ok(Some(format!(
                "report exceeds report field limits ({:?} has length of {} characters \
                    while the limit is {})",
                field,
                length,
                field.max_length()
            )));
        }

        // Calculate attachments size.
        let mut attachments_size_in_bytes: usize = 0;
        for attachment in attachments.iter() {
            attachments_size_in_bytes += attachment.data.len();
        }

        // Log event.
        self.logger.lock().unwrap().print_and_log(
            LogCategory::Info,
            &format!(
                "received a report (attachments size ~{} KB) from reporter {}",
                attachments_size_in_bytes / 1024,
                self.socket_addr
            ),
        );

        {
            if let Err(app_error) = self
                .database
                .lock()
                .unwrap()
                .save_report(game_report, attachments)
            {
                let result_code = ReportResult::InternalError;

                // Notify reporter of our failure.
                if let Some(app_error) = send_message(
                    &mut self.socket,
                    &self.secret_key,
                    ReporterAnswer::Report { result_code },
                ) {
                    return Err(app_error);
                }

                return Err(app_error);
            }
        }

        self.logger.lock().unwrap().print_and_log(
            LogCategory::Info,
            &format!("saved a report from reporter {}", self.socket_addr),
        );

        // Answer "OK".
        if let Some(app_error) = send_message(
            &mut self.socket,
            &self.secret_key,
            ReporterAnswer::Report {
                result_code: ReportResult::Ok,
            },
        ) {
            return Err(app_error);
        }

        Ok(None)
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

    /// Processes reporter's attachment size request.
    fn handle_attachment_size_query_request(&mut self) -> Option<AppError> {
        // Log event.
        self.logger.lock().unwrap().print_and_log(
            LogCategory::Info,
            &format!(
                "received maximum attachment size request from reporter {}",
                self.socket_addr
            ),
        );

        let answer = ReporterAnswer::MaxAttachmentSize {
            max_attachments_size_in_mb: self.max_total_attachment_size_in_mb,
        };
        if let Some(app_error) = send_message(&mut self.socket, &self.secret_key, answer) {
            return Some(app_error);
        }

        None
    }
}

impl Drop for ReporterService {
    /// Logs information about connection being closed.
    fn drop(&mut self) {
        let user_info = format!("closing connection with reporter {}", self.socket_addr);

        let mut exit_reason = String::new();
        if self.exit_error.is_some() {
            let error = self.exit_error.as_ref().unwrap();

            if let Err(app_error) = error {
                exit_reason = format!(" due to internal error (bug):\n{}", app_error);
            } else {
                exit_reason = format!(", reason: {}", error.as_ref().unwrap());
            }
        }

        let mut connected_count = self.connected_count.lock().unwrap();
        *connected_count -= 1;
        let message = format!(
            "{}{}\n------------------------- [connected: {}] -------------------------",
            user_info, exit_reason, connected_count
        );

        self.logger
            .lock()
            .unwrap()
            .print_and_log(LogCategory::Info, &message);
    }
}
