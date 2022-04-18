// Std.
use std::fs::{File, *};
use std::io::prelude::*;
use std::path::Path;

// External.
use chrono::Local;

pub const LOG_FILE_NAME: &str = "server.log";

pub enum LogCategory {
    Info,
    Warning,
    Error,
}

pub struct Logger;

impl Logger {
    /// Removes old log file and creates an empty one.
    pub fn new() -> Self {
        Logger::recreate_log_file();
        Self {}
    }
    pub fn print_and_log(&self, category: LogCategory, text: &str) {
        let mut message = String::new();

        match category {
            LogCategory::Info => message += "INFO: ",
            LogCategory::Warning => message += "WARNING: ",
            LogCategory::Error => message += "ERROR: ",
        }

        message += text;
        if !message.ends_with('.') {
            message += ".";
        }

        let mut log_file = self.open_log_file();

        let datetime = Local::now();

        if let Err(e) = writeln!(log_file, "[{}]: {}", datetime.naive_local(), message) {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        } else {
            println!("[{}]: {}", datetime.naive_local(), message);
        }
    }
    fn open_log_file(&self) -> std::fs::File {
        let log_file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(LOG_FILE_NAME);
        if let Err(e) = log_file {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        }

        log_file.unwrap()
    }
    fn recreate_log_file() {
        let mut log_path = String::from(std::env::current_dir().unwrap().to_str().unwrap());

        // Check ending.
        #[cfg(target_os = "linux")]
        {
            if !log_path.ends_with('/') {
                log_path += "/";
            }
        }
        #[cfg(target_os = "windows")]
        {
            if !log_path.ends_with('\\') {
                log_path += "\\";
            }
        }

        log_path += "logs";

        // Ending.
        #[cfg(target_os = "linux")]
        {
            log_path += "/";
        }
        #[cfg(target_os = "windows")]
        {
            log_path += "\\";
        }

        if !Path::new(&log_path).exists() {
            if let Err(e) = create_dir(&log_path) {
                panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
            }
        } else {
            Logger::remove_oldest_log_if_needed(&log_path);
        }

        let local = Local::now();

        let log_path = log_path
            + &format!("{}_", local.format("%Y-%m-%d_%H:%M:%S").to_string())
            + LOG_FILE_NAME;

        // Remove log file if exists.
        if Path::new(&log_path).exists() {
            if let Err(e) = remove_file(&log_path) {
                panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
            }
        }

        // Create log file.
        let log_file = File::create(&log_path);
        if let Err(e) = log_file {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        }
    }
    fn remove_oldest_log_if_needed(log_path: &str) {
        let paths = std::fs::read_dir(log_path);
        if let Err(e) = paths {
            panic!("{}", e);
        }
        let paths = paths.unwrap();

        for path in paths {
            if let Err(e) = path {
                panic!("{}", e);
            }
            let path = path.unwrap();

            if path.file_type().unwrap().is_file() {
                // TODO:
                // let metadata = fs::metadata("foo.txt")
            }
        }
    }
}
