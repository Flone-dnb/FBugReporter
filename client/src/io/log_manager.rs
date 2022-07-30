// Std.
use std::fs::{File, *};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

// External.
use chrono::Local;
use platform_dirs::UserDirs;

const LOG_FILE_NAME: &str = "client.log";
const LOG_FILE_DIR: &str = "FBugReporter";

pub struct LogManager {
    log_file_path: PathBuf,
}

impl LogManager {
    /// Removes old log file and creates an empty one.
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_log_file_path() -> PathBuf {
        #[cfg(any(windows, unix))]
        {
            let user_dirs = UserDirs::new().expect(&format!(
                "An error occurred at [{}, {}]: can't read user dirs.",
                file!(),
                line!(),
            ));

            let mut log_path = user_dirs.document_dir;

            log_path.push(LOG_FILE_DIR);

            // Create directory if not exists.
            if !log_path.exists() {
                if let Err(e) = create_dir_all(&log_path) {
                    panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
                }
            }

            log_path.push(LOG_FILE_NAME);
            log_path
        }
        #[cfg(not(any(windows, unix)))]
        {
            compile_error!("Reporter is not implemented for this OS.");
        }
    }
    pub fn log(&self, text: &str) {
        let mut log_file = self.open_log_file();

        let datetime = Local::now();

        if let Err(e) = writeln!(log_file, "[{}]: {}", datetime.naive_local(), text) {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        }
    }
    fn open_log_file(&self) -> std::fs::File {
        let log_file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.log_file_path);
        if let Err(e) = log_file {
            panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
        }

        log_file.unwrap()
    }
    /// Returns new log file path.
    fn recreate_log_file() -> PathBuf {
        let log_path = LogManager::get_log_file_path();

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
}

impl Default for LogManager {
    fn default() -> Self {
        Self {
            log_file_path: LogManager::recreate_log_file(),
        }
    }
}
