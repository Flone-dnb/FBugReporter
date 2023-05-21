// Std.
use std::fs::{File, *};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

// External.
use chrono::Local;
use platform_dirs::UserDirs;

pub const LOG_FILE_NAME: &str = "server.log";
const LOG_DIR_PREFIX: &str = "FBugReporter";
const LOG_DIR: &str = "server_logs";
const MAX_LOG_FILE_COUNT: usize = 10;

pub enum LogCategory {
    Info,
    Warning,
    Error,
}

pub struct LogManager {
    current_log_file: PathBuf,
}

impl LogManager {
    /// Removes old log file and creates an empty one.
    pub fn new() -> Self {
        Self {
            current_log_file: LogManager::recreate_log_file(),
        }
    }
    /// Prints text on the screen and writes it to log file.
    pub fn print_and_log(&self, category: LogCategory, text: &str) {
        let mut message = match category {
            LogCategory::Info => String::from("INFO: "),
            LogCategory::Warning => String::from("WARNING: "),
            LogCategory::Error => String::from("ERROR: "),
        };
        message += text;

        let mut log_file = self.open_log_file();
        let datetime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        if let Err(e) = writeln!(log_file, "[{}] {}", datetime, message) {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        } else {
            println!("[{}] {}", datetime, message);
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
    fn recreate_log_file() -> PathBuf {
        let mut log_path;

        #[cfg(any(unix, windows))]
        {
            let user_dirs = UserDirs::new().unwrap_or_else(|| {
                panic!(
                    "An error occurred at [{}, {}]: can't read user dirs.",
                    file!(),
                    line!(),
                )
            });

            log_path = user_dirs.document_dir;
        }

        #[cfg(not(any(unix, windows)))]
        {
            compile_error!("Server is not implemented for this OS.");
        }

        log_path.push(LOG_DIR_PREFIX);
        log_path.push(LOG_DIR);

        if !log_path.exists() {
            if let Err(e) = create_dir_all(&log_path) {
                panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
            }
        } else {
            LogManager::remove_oldest_log_if_needed(&log_path);
        }

        let local = Local::now();

        let filename = format!("{}_{}", local.format("%Y-%m-%d_%H-%M-%S"), LOG_FILE_NAME);

        log_path.push(&filename);

        // Remove log file if exists.
        if log_path.exists() {
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
    #[allow(clippy::print_literal)] // TODO: remove when https://github.com/rust-lang/rust-clippy/issues/2768 is fixed
    fn remove_oldest_log_if_needed(log_path: &Path) {
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
