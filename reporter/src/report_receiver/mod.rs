// Std.
use std::str::FromStr;

// External.
use strum_macros::{Display, EnumString};

// Custom.
use self::report_receiver_server::ReportReceiverServer;
use crate::log_manager::LogManager;
use shared::misc::report::*;

pub mod report_receiver_server;

/// Type of the remote "server" that we send our reports to.
#[derive(Debug, EnumString, Display)]
pub enum ReportReceiverType {
    /// FBugReporter server.
    Server,
}

/// Result of the `send_report` operation.
pub enum SendReportResult {
    /// Sent the report successfully.
    Ok,
    /// Unable to connect to the server (not found or inactive).
    CouldNotConnect,
    /// Error message without the call stack.
    /// Implementators of "report receiver" trait are recommended to log an error message with the
    /// full call stack before returning the error message.
    Other(String),
}

pub trait ReportReceiver {
    /// Requests maximum allowed size of attachments (in total) in megabytes.
    ///
    /// ## Remarks
    /// This function is generally used to quickly check the attachment size before
    /// sending it - to know whether our request will fail or not.
    /// Once your report is sent, the report receiver will also check this on its side.
    ///
    /// ## Arguments
    /// * `server_addr`: address of the server to connect to.
    /// * `logger`: logger to use.
    ///
    /// ## Return
    /// `None` if this receiver does not provide such functionality or something went wrong
    /// (see logs), otherwise maximum allowed size of attachments in megabytes.
    fn request_max_attachment_size_in_mb(
        &mut self,
        remote_address: String,
        logger: &mut LogManager,
    ) -> Option<usize>;

    /// Sends the specified report to the specified remote address.
    ///
    /// ## Arguments
    /// * `remote_address` string that describes remote entity's address (depends on the report
    /// receiver), this can be a domain name, IPv4 address, GitHub user/repo combination or
    /// something else.
    /// * `auth_token` optional authentication token that some report receivers require.
    /// * `report` report to send.
    /// * `logger` logger that will be used to write to logs.
    /// * `attachments` report attachements.
    fn send_report(
        &mut self,
        remote_address: String,
        auth_token: String,
        report: GameReport,
        logger: &mut LogManager,
        attachments: Vec<ReportAttachment>,
    ) -> SendReportResult;
}

/// Creates a new report receiver by parsing the specified report receiver type.
///
/// ## Return
/// `None` if the specified receiver type string is invalid, otherwise created report receiver.
pub fn create_report_receiver(receiver_type: &str) -> Option<Box<dyn ReportReceiver>> {
    // Parse type.
    let receiver_type = ReportReceiverType::from_str(receiver_type);
    if receiver_type.is_err() {
        return None;
    }
    let receiver_type = receiver_type.unwrap();

    // Create object.
    match receiver_type {
        ReportReceiverType::Server => Some(Box::new(ReportReceiverServer::new())),
    }
}
