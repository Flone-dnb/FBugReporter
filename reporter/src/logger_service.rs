// Std.
use std::fs::{File, *};
use std::io::prelude::*;
use std::path::Path;

// External.
#[cfg(target_os = "windows")]
use platform_dirs::UserDirs;

const LOG_FILE_NAME: &str = "FBugReporter - reporter.log";
#[cfg(target_os = "windows")]
const LOG_FILE_DIR: &str = "FBugReporter";

pub struct Logger {
    log_file_path: String,
}

impl Logger {
    /// Removes old log file and creates an empty one.
    pub fn new() -> Self {
        Self {
            log_file_path: Logger::recreate_log_file(),
        }
    }
    pub fn log(&self, text: &str) {
        let mut log_file = self.open_log_file();

        if let Err(e) = writeln!(log_file, "{}", text) {
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
    fn recreate_log_file() -> String {
        #[cfg(target_os = "linux")]
        {
            let mut log_path = String::from(std::env::current_dir().unwrap().to_str().unwrap());

            // Check ending.
            if !log_path.ends_with('/') {
                log_path += "/";
            }

            let log_path = log_path + LOG_FILE_NAME;

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

            return log_path;
        }
        #[cfg(target_os = "windows")]
        {
            let user_dirs = UserDirs::new();
            if user_dirs.is_none() {
                panic!(
                    "An error occurred at [{}, {}]: can't read user dirs.",
                    file!(),
                    line!(),
                );
            }
            let user_dirs = user_dirs.unwrap();

            // Get Documents folder.
            let mut log_path = String::from(user_dirs.document_dir.to_str().unwrap());

            // Check ending.
            if !log_path.ends_with('\\') {
                log_path += "\\";
            }

            log_path += LOG_FILE_DIR;
            log_path += "\\";

            // Create directory if not exists.
            if !Path::new(&log_path).exists() {
                if let Err(e) = create_dir(&log_path) {
                    panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
                }
            }

            log_path += LOG_FILE_NAME;

            // Create log file.
            let log_file = File::create(&log_path);
            if let Err(e) = log_file {
                panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
            }

            return log_path;
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            compile_error!("Reporter is not implemented for this OS.");
        }
    }
}
