// Std.
use backtrace::Backtrace;
use std::net::{Ipv4Addr, SocketAddrV4};

// External.
use gdnative::prelude::*;

// Custom.
mod logger_service;
mod misc;
mod reporter_service;
use logger_service::*;
use misc::*;
use reporter_service::*;

#[derive(NativeClass)]
#[inherit(Node)]
struct Reporter {
    server_addr: Option<SocketAddrV4>,
    last_report: Option<GameReport>,
    last_error: String,
}

#[gdnative::methods]
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
    fn send_report(
        &mut self,
        _owner: &Node,
        report_name: String,
        report_text: String,
        sender_name: String,
        sender_email: String,
        game_name: String,
        game_version: String,
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
        let mut logger = Logger::new();
        logger.log(&format!("Received a report: {:?}", report));

        let mut reporter = ReporterService::new();
        let (result_code, error_message) =
            reporter.send_report(self.server_addr.unwrap(), &mut logger);

        if result_code == ReportResult::Ok {
            // Save report.
            self.last_report = Some(report);
        } else {
            if error_message.is_none() {
                self.last_error = String::from("Error message is None.");
            } else {
                self.last_error = error_message.unwrap();
            }
        }

        return result_code.value();
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
