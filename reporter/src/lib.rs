#![deny(warnings)]

// Std.
use backtrace::Backtrace;
use std::fs::metadata;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::{env, fs::File};

// External.
use gdnative::prelude::*;
use image::{ImageBuffer, RgbaImage};

// Custom.
mod log_manager;
mod reporter_service;
use log_manager::*;
use reporter_service::*;
use shared::misc::{error::AppError, report::*};

#[derive(NativeClass, Default)]
#[inherit(Node)]
struct Reporter {
    report_name: String,
    report_text: String,
    sender_name: String,
    sender_email: String,
    game_name: String,
    game_version: String,
    attachments: Vec<String>,
    server_addr: Option<String>,
    screenshot_path: Option<String>,
    last_report: Option<GameReport>,
    last_error: String,
}

#[methods]
impl Reporter {
    fn new(_owner: &Node) -> Self {
        Self::default()
    }

    #[export]
    fn _ready(&self, _owner: &Node) {}

    #[export]
    fn set_report_name(&mut self, _owner: &Node, report_name: String) {
        self.report_name = report_name;
    }

    #[export]
    fn set_report_text(&mut self, _owner: &Node, report_text: String) {
        self.report_text = report_text;
    }

    #[export]
    fn set_sender_name(&mut self, _owner: &Node, sender_name: String) {
        self.sender_name = sender_name;
    }

    #[export]
    fn set_sender_email(&mut self, _owner: &Node, sender_email: String) {
        self.sender_email = sender_email;
    }

    #[export]
    fn set_game_name(&mut self, _owner: &Node, game_name: String) {
        self.game_name = game_name;
    }

    #[export]
    fn set_game_version(&mut self, _owner: &Node, game_version: String) {
        self.game_version = game_version;
    }

    #[export]
    fn set_report_attachments(&mut self, _owner: &Node, attachments: Vec<String>) {
        self.attachments = attachments;
    }

    #[export]
    fn set_clear_screenshot(&mut self, _owner: &Node) {
        self.screenshot_path = None;
    }

    #[export]
    fn get_log_file_path(&self, _owner: &Node) -> String {
        LogManager::get_log_file_path().to_str().unwrap().to_owned()
    }

    #[export]
    fn get_last_error(&mut self, _owner: &Node) -> String {
        self.last_error.clone()
    }

    #[export]
    fn set_server(&mut self, _owner: &Node, server: String, port: u16) {
        self.server_addr = Some(format!("{}:{}", server, port));
    }

    #[export]
    fn set_screenshot(&mut self, _owner: &Node, viewport_image: Ref<Image>) {
        // Prepare screenshot path.
        let mut screenshot_path_buf = env::temp_dir();
        screenshot_path_buf.push("FBugReporter");
        screenshot_path_buf.push("reporter");

        if let Err(e) = std::fs::create_dir_all(screenshot_path_buf.as_path()) {
            godot_warn!("{}", AppError::new(&e.to_string()).to_string());
            return;
        }

        screenshot_path_buf.push("screenshot.jpg");

        // Prepare image.
        let viewport_image: TRef<Image> = unsafe { viewport_image.assume_safe() };

        let mut img: RgbaImage = ImageBuffer::new(
            viewport_image.get_width() as u32,
            viewport_image.get_height() as u32,
        );

        // Write pixels from viewport image.
        viewport_image.lock();
        for row in 0..viewport_image.get_height() {
            for column in 0..viewport_image.get_width() {
                let color: Color = viewport_image.get_pixel(column, row);
                let new_pixel = image::Rgba([
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    (color.a * 255.0) as u8,
                ]);

                img.put_pixel(column as u32, row as u32, new_pixel);
            }
        }
        viewport_image.unlock();

        // Save image.
        if let Err(e) = img.save(screenshot_path_buf.as_path()) {
            godot_warn!("{}", AppError::new(&e.to_string()).to_string());
        } else {
            let screenshot_path = screenshot_path_buf.as_path().to_str();
            match screenshot_path {
                Some(screenshot_path) => {
                    self.screenshot_path = Some(String::from(screenshot_path));
                }
                None => {
                    godot_warn!(
                        "{}",
                        AppError::new("unable to convert screenshot path to string")
                    );
                }
            }
        }
    }

    #[export]
    fn get_last_modified_files(
        &self,
        _owner: &Node,
        path: String,
        file_count: usize,
    ) -> Vec<String> {
        let paths = std::fs::read_dir(path);
        if let Err(ref e) = paths {
            godot_warn!("{}", AppError::new(&e.to_string()));
            return Vec::new();
        }
        let paths = paths.unwrap();

        let mut files: Vec<(PathBuf, u64)> = Vec::new();

        // Read files modification date.
        for path in paths {
            if let Err(ref e) = path {
                godot_warn!("{}", AppError::new(&e.to_string()));
                continue;
            }
            let path = path.unwrap();

            if path.file_type().unwrap().is_file() {
                let metadata = metadata(path.path());
                if let Err(ref e) = metadata {
                    godot_warn!("{}", AppError::new(&e.to_string()));
                    continue;
                }
                let metadata = metadata.unwrap();

                let last_modified = metadata.modified();
                if let Err(e) = last_modified {
                    godot_warn!("{}", AppError::new(&e.to_string()));
                    continue;
                }

                let elapsed_seconds = last_modified.unwrap().elapsed();
                if let Err(e) = elapsed_seconds {
                    godot_warn!("{}", AppError::new(&e.to_string()));
                    continue;
                }

                files.push((path.path(), elapsed_seconds.unwrap().as_secs()));
            }
        }

        files.sort_by(|a, b| a.1.cmp(&b.1));

        let mut out_paths: Vec<String> = Vec::new();
        for file in files {
            let path = file.0.as_path().to_str();
            match path {
                Some(path) => {
                    out_paths.push(String::from(path));
                    if out_paths.len() == file_count {
                        break;
                    }
                }
                None => {
                    godot_warn!("{}", AppError::new("unable to convert path to string"));
                }
            }
        }

        out_paths
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
    fn send_report(&mut self, _owner: &Node) -> i32 {
        if self.server_addr.is_none() {
            return ReportResult::ServerNotSet.value();
        }

        // Construct report object.
        let report = GameReport {
            report_name: self.report_name.clone(),
            report_text: self.report_text.clone(),
            sender_name: self.sender_name.clone(),
            sender_email: self.sender_email.clone(),
            game_name: self.game_name.clone(),
            game_version: self.game_version.clone(),
            client_os_info: os_info::get(),
        };

        // Check input length.
        let invalid_field = self.is_input_valid(&report);
        if let Some(report_limit_error) = invalid_field {
            self.last_error = report_limit_error.id().to_string();
            return ReportResult::InvalidInput.value();
        }

        // Prepare logging.
        let mut logger = LogManager::new();
        logger.log(&format!(
            "FBugReporter (reporter) (v{})",
            env!("CARGO_PKG_VERSION"),
        ));
        logger.log(&format!("Received a report: {:?}", report));

        // Add screenshot as an attachment.
        if let Some(screenshot_path) = &self.screenshot_path {
            if !Path::new(&screenshot_path).exists() {
                godot_warn!(
                    "{}",
                    AppError::new("previously saved screenshot no longer exists")
                );
            } else if self
                .attachments
                .iter()
                .find(|&path| path == screenshot_path)
                == None
            {
                self.attachments.push(screenshot_path.clone());
            }
        } else {
            logger.log("No screenshot provided.");
        }

        let mut reporter = ReporterService::new();

        // Process other attachments.
        let mut report_attachments: Vec<ReportAttachment> = Vec::new();
        if !self.attachments.is_empty() {
            // Check that the specified paths exist.
            for path in self.attachments.iter() {
                if !Path::new(&path).exists() {
                    return ReportResult::AttachmentDoesNotExist.value();
                }
            }

            // Request mac attachment size (in total) in MB.
            let result = reporter.request_max_attachment_size_in_mb(
                self.server_addr.as_ref().unwrap().clone(),
                &mut logger,
            );
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
                self.attachments.clone(),
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
            self.server_addr.as_ref().unwrap().clone(),
            report.clone(),
            &mut logger,
            report_attachments,
        );

        if result_code == ReportResult::Ok {
            // Save report.
            self.last_report = Some(report);
            logger.log("Successfully sent the report.");

            // Cleanup.
            if let Some(screenshot_path) = self.screenshot_path.take() {
                if Path::new(&screenshot_path).exists() {
                    if let Err(e) = std::fs::remove_file(&screenshot_path) {
                        logger.log(&format!(
                            "failed to delete screenshot from \"{}\" (error: {})",
                            &screenshot_path, e
                        ));
                    }
                }
            }
        } else {
            match error_message {
                Some(app_error) => {
                    logger.log(&app_error.to_string());
                    self.last_error = app_error.get_message();
                }
                None => {
                    self.last_error =
                        String::from("An error occurred but the error message is empty.");
                    logger.log(&self.last_error);
                }
            }
        }

        result_code.value()
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

        Ok(attachments)
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

        None
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
