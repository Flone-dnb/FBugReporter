// Std.
use backtrace::Backtrace;
use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::Path;

// External.
use gdnative::prelude::*;

// Custom.
mod log_manager;
mod reporter_service;
use log_manager::*;
use reporter_service::*;
use shared::misc::report::*;

#[derive(NativeClass)]
#[inherit(Node)]
struct Reporter {
    server_addr: Option<SocketAddrV4>,
    last_report: Option<GameReport>,
    last_error: String,
}

#[methods]
impl Reporter {
    fn new(_owner: &Node) -> Self {
        Reporter {
            server_addr: None,
            last_report: None,
            last_error: String::new(),
        }
    }

    #[export]
    fn _ready(&self, _owner: &Node) {}

    #[export]
    fn get_log_file_path(&self, _owner: &Node) -> String {
        LogManager::get_log_file_path()
    }

    #[export]
    fn get_last_error(&mut self, _owner: &Node) -> String {
        return self.last_error.clone();
    }

    #[export]
    fn set_server(&mut self, _owner: &Node, ip_a: u8, ip_b: u8, ip_c: u8, ip_d: u8, port: u16) {
        self.server_addr = Some(SocketAddrV4::new(
            Ipv4Addr::new(ip_a, ip_b, ip_c, ip_d),
            port,
        ));
    }

    #[export]
    fn get_field_limit(&mut self, _owner: &Node, field_id: u64) -> i32 {
        if ReportLimits::ReportName.id() == field_id {
            ReportLimits::ReportName.max_length() as i32
        } else if ReportLimits::ReportText.id() == field_id {
            ReportLimits::ReportText.max_length() as i32
        } else if ReportLimits::SenderName.id() == field_id {
            ReportLimits::SenderName.max_length() as i32
        } else if ReportLimits::SenderEMail.id() == field_id {
            ReportLimits::SenderEMail.max_length() as i32
        } else if ReportLimits::GameName.id() == field_id {
            ReportLimits::GameName.max_length() as i32
        } else if ReportLimits::GameVersion.id() == field_id {
            ReportLimits::GameVersion.max_length() as i32
        } else {
            0
        }
    }

    #[export]
    fn send_report(
        &mut self,
        _owner: &Node,
        report_name: String,
        report_text: String,
        sender_name: String,
        sender_email: String,
        game_name: String,
        game_version: String,
        attachments: Vec<String>,
    ) -> i32 {
        if self.server_addr.is_none() {
            return ReportResult::ServerNotSet.value();
        }

        // Construct report object.
        let report = GameReport {
            report_name,
            report_text,
            sender_name,
            sender_email,
            game_name,
            game_version,
            client_os_info: os_info::get(),
        };

        // Check input length.
        let invalid_field = self.is_input_valid(&report);
        if invalid_field.is_some() {
            self.last_error = invalid_field.unwrap().id().to_string();
            return ReportResult::InvalidInput.value();
        }

        // Prepare logging.
        let mut logger = LogManager::new();
        logger.log(&format!("Received a report: {:?}", report));

        let mut reporter = ReporterService::new();

        // Generate report attachments (if needed).
        let mut report_attachments: Vec<ReportAttachment> = Vec::new();
        if !attachments.is_empty() {
            // Check that the specified paths exist.
            for path in attachments.iter() {
                if !Path::new(&path).exists() {
                    return ReportResult::AttachmentDoesNotExist.value();
                }
            }

            // Request mac attachment size (in total) in MB.
            let result =
                reporter.request_max_attachment_size_in_mb(self.server_addr.unwrap(), &mut logger);
            if let Err(app_error) = result {
                self.last_error = app_error.get_message();
                logger.log(&app_error.to_string());
                return ReportResult::InternalError.value();
            }
            let max_attachments_size_in_mb = result.unwrap();

            logger.log(&format!(
                "Received maximum allowed attachment size of {} MB.",
                max_attachments_size_in_mb
            ));

            // Generate attachments from paths.
            let result = Self::generate_attachments_from_paths(
                attachments,
                max_attachments_size_in_mb,
                &mut logger,
            );
            if let Err((result, msg)) = result {
                self.last_error = msg.clone();
                logger.log(&msg);
                return result.value();
            }
            report_attachments = result.unwrap();
        }

        let (result_code, error_message) = reporter.send_report(
            self.server_addr.unwrap(),
            report.clone(),
            &mut logger,
            report_attachments,
        );

        if result_code == ReportResult::Ok {
            // Save report.
            self.last_report = Some(report);
            logger.log("Successfully sent the report.");
        } else {
            if error_message.is_none() {
                self.last_error = String::from("An error occurred but the error message is empty.");
                logger.log(&self.last_error);
            } else {
                let app_error = error_message.unwrap();
                logger.log(&app_error.to_string());
                self.last_error = app_error.get_message();
            }
        }

        return result_code.value();
    }

    /// Generates ReportAttachment from file paths.
    ///
    /// Expects file path to exist.
    fn generate_attachments_from_paths(
        paths: Vec<String>,
        max_attachments_size_in_mb: usize,
        logger: &mut LogManager,
    ) -> Result<Vec<ReportAttachment>, (ReportResult, String)> {
        let mut attachments: Vec<ReportAttachment> = Vec::new();
        let mut total_attachment_size_in_bytes: usize = 0;
        for path in paths {
            let file_path = Path::new(&path);

            // Check file name.
            let file_name = file_path.file_name();
            if file_name.is_none() {
                return Err((
                    ReportResult::InternalError,
                    format!(
                        "An error occurred at [{}, {}]: file name is empty ({})",
                        file!(),
                        line!(),
                        path
                    ),
                ));
            }
            let file_name = file_name.unwrap().to_str();
            if file_name.is_none() {
                return Err((
                    ReportResult::InternalError,
                    format!(
                        "An error occurred at [{}, {}]: failed to get file name ({})",
                        file!(),
                        line!(),
                        path
                    ),
                ));
            }
            let file_name = String::from(file_name.unwrap());
            total_attachment_size_in_bytes += file_name.len();

            logger.log(&format!("Processing report attachment {}...", file_name,));

            // Read file into vec.
            let mut data: Vec<u8> = Vec::new();

            let file = File::open(path);
            if let Err(e) = file {
                return Err((
                    ReportResult::InternalError,
                    format!("An error occurred at [{}, {}]: {}", file!(), line!(), e),
                ));
            }
            let mut file = file.unwrap();
            let result = file.read_to_end(&mut data);
            if let Err(e) = result {
                return Err((
                    ReportResult::InternalError,
                    format!("An error occurred at [{}, {}]: {}", file!(), line!(), e),
                ));
            }
            let file_size = result.unwrap();
            total_attachment_size_in_bytes += file_size;

            logger.log(&format!(
                "Processed report attachment {} of size {} bytes.",
                file_name, file_size
            ));

            let attachment = ReportAttachment { file_name, data };
            attachments.push(attachment);
        }

        let max_attachments_size_in_bytes = max_attachments_size_in_mb * 1024 * 1024;
        if total_attachment_size_in_bytes > max_attachments_size_in_bytes {
            return Err((
                ReportResult::AttachmentTooBig,
                format!(
                    "An error occurred at [{}, {}]: maximum attachment size exceeded ({} > {})",
                    file!(),
                    line!(),
                    total_attachment_size_in_bytes,
                    max_attachments_size_in_bytes
                ),
            ));
        }

        return Ok(attachments);
    }

    /// Returns the id of the invalid field.
    fn is_input_valid(&self, report: &GameReport) -> Option<ReportLimits> {
        if report.report_name.chars().count() > ReportLimits::ReportName.max_length() {
            return Some(ReportLimits::ReportName);
        }

        if report.report_text.chars().count() > ReportLimits::ReportText.max_length() {
            return Some(ReportLimits::ReportText);
        }

        if report.sender_name.chars().count() > ReportLimits::SenderName.max_length() {
            return Some(ReportLimits::SenderName);
        }

        if report.sender_email.chars().count() > ReportLimits::SenderEMail.max_length() {
            return Some(ReportLimits::SenderEMail);
        }

        if report.game_name.chars().count() > ReportLimits::GameName.max_length() {
            return Some(ReportLimits::GameName);
        }

        if report.game_version.chars().count() > ReportLimits::GameVersion.max_length() {
            return Some(ReportLimits::GameVersion);
        }

        return None;
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<Reporter>();

    init_panic_hook();
}

godot_init!(init);

pub fn init_panic_hook() {
    // To enable backtrace, you will need the `backtrace` crate to be included in your cargo.toml, or
    // a version of Rust where backtrace is included in the standard library (e.g. Rust nightly as of the date of publishing)
    // use backtrace::Backtrace;
    // use std::backtrace::Backtrace;
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let loc_string;
        if let Some(location) = panic_info.location() {
            loc_string = format!("file '{}' at line {}", location.file(), location.line());
        } else {
            loc_string = "unknown location".to_owned()
        }

        let error_message;
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            error_message = format!("[RUST] {}: panic occurred: {:?}", loc_string, s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            error_message = format!("[RUST] {}: panic occurred: {:?}", loc_string, s);
        } else {
            error_message = format!("[RUST] {}: unknown panic occurred", loc_string);
        }
        godot_error!("{}", error_message);

        // Uncomment the following line if backtrace crate is included as a dependency
        godot_error!("Backtrace:\n{:?}", Backtrace::new());
        (*(old_hook.as_ref()))(panic_info);

        // don't call the actual assert (plus less work for devs)
        // FBugReporter should never crash the game

        // unsafe {
        //     if let Some(gd_panic_hook) =
        //         gdnative::api::utils::autoload::<gdnative::api::Node>("rust_panic_hook")
        //     {
        //         gd_panic_hook.call(
        //             "rust_panic_hook",
        //             &[GodotString::from_str(error_message).to_variant()],
        //         );
        //     }
        // }
    }));
}
