// Std.
use std::fs::{File, *};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

// External.
use chrono::Local;

pub const LOG_FILE_NAME: &str = "server.log";
const LOG_DIR: &str = "logs";
const MAX_LOG_FILE_COUNT: usize = 10;

pub enum LogCategory {
    Info,
    Warning,
    Error,
}

pub struct Logger {
    current_log_file: String,
}

impl Logger {
    /// Removes old log file and creates an empty one.
    pub fn new() -> Self {
        Self {
            current_log_file: Logger::recreate_log_file(),
        }
    }
    /// Prints text on the screen and writes it to log file.
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
    /// Opens log file for writing.
    fn open_log_file(&self) -> std::fs::File {
        let log_file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.current_log_file);
        if let Err(e) = log_file {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        }

        log_file.unwrap()
    }
    /// Removes log file (if exists) and creates a new one.
    /// Returns path to the new log file.
    fn recreate_log_file() -> String {
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

        log_path += LOG_DIR;

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

        let filename = format!("{}_{}", local.format("%Y-%m-%d_%H-%M-%S"), LOG_FILE_NAME);

        log_path += &filename;

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

        log_path
    }
    /// Removes the oldest log file if there are `MAX_LOG_FILE_COUNT` log files or more.
    fn remove_oldest_log_if_needed(log_path: &str) {
        let paths = std::fs::read_dir(log_path);
        if let Err(ref e) = paths {
            println!("ERROR: {} at [{}, {}]", e, file!(), line!());
            return;
        }
        let paths = paths.unwrap();

        let mut logs: Vec<(PathBuf, u64)> = Vec::new();

        for path in paths {
            if let Err(ref e) = path {
                println!("ERROR: {} at [{}, {}]", e, file!(), line!());
                continue;
            }
            let path = path.unwrap();

            if path.file_type().unwrap().is_file() {
                let metadata = metadata(path.path());
                if let Err(ref e) = metadata {
                    println!("ERROR: {} at [{}, {}]", e, file!(), line!());
                    continue;
                }
                let metadata = metadata.unwrap();
                let last_modified = metadata.modified();
                if let Err(e) = last_modified {
                    println!("ERROR: {} at [{}, {}]", e, file!(), line!());
                    continue;
                }
                let elapsed_seconds = last_modified.unwrap().elapsed();
                if let Err(e) = elapsed_seconds {
                    println!("ERROR: {} at [{}, {}]", e, file!(), line!());
                    continue;
                }
                logs.push((path.path(), elapsed_seconds.unwrap().as_secs()));
            }
        }

        if logs.is_empty() {
            return;
        }

        if logs.len() < MAX_LOG_FILE_COUNT {
            return;
        }

        logs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Remove the oldest log file.
        if let Err(e) = remove_file(logs.last().unwrap().0.clone()) {
            println!("ERROR: failed to remove oldest log file, error: {}", e);
        }
    }
}
