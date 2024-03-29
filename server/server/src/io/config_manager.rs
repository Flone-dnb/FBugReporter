// Std.
use std::{fs::create_dir_all, ops::Range, path::PathBuf, str::FromStr};

// External.
use configparser::ini::Ini;
use platform_dirs::{AppDirs, UserDirs};
use rand::Rng;

// Custom.
use super::log_manager::LOG_FILE_NAME;
use shared::misc::error::AppError;

const RANDOM_PORT_RANGE: Range<u16> = 7000..65535;

const DEFAULT_MAX_ALLOWED_LOGIN_ATTEMPTS: u32 = 3;
const DEFAULT_BAN_TIME_DURATION_IN_MIN: i64 = 5;
const DEFAULT_MAX_ATTACHMENT_SIZE_IN_MB: usize = 5;

const CONFIG_FILE_DIR: &str = "FBugReporter";
const CONFIG_FILE_NAME: &str = "server_config.ini";

// --------------- server section start ---------------
const CONFIG_SERVER_SECTION_NAME: &str = "server";
const CONFIG_PORT_REPORTER_PARAM: &str = "port_for_reporters";
const CONFIG_PORT_CLIENT_PARAM: &str = "port_for_clients";
const CONFIG_MAX_ATTACHMENT_SIZE_IN_MB_PARAM: &str = "max_total_attachment_size_in_mb";
// --------------- server section end ---------------
// --------------- login section start ---------------
const CONFIG_LOGIN_SECTION_NAME: &str = "login";
const CONFIG_MAX_ALLOWED_LOGIN_ATTEMPTS_PARAM: &str = "max_allowed_login_attempts_until_ban";
const CONFIG_BAN_TIME_DURATION_IN_MIN: &str = "ban_time_duration_in_min";
// --------------- login section end ---------------

#[derive(Debug)]
pub struct ConfigManager {
    pub port_for_reporters: u16,
    pub port_for_clients: u16,
    pub max_attachment_size_in_mb: usize,
    pub max_allowed_login_attempts: u32,
    pub ban_time_duration_in_min: i64,
    pub config_file_path: PathBuf,
    pub log_file_path: PathBuf,
}

impl ConfigManager {
    /// Reads config values from .ini file if it exists,
    /// otherwise using default values and creating a new config .ini file.
    pub fn new() -> Self {
        let mut server_config = ConfigManager::default();

        // Try reading config from .ini file.
        let mut config = Ini::new();
        let map = config.load(&server_config.config_file_path);
        if map.is_err() {
            println!(
                "INFO: could not open the config file \"{0}\", using default values \
                and creating a new \"{0}\" configuration file.",
                CONFIG_FILE_NAME
            );
            // No file found, create a new file.
            if let Err(e) = server_config.save_config() {
                // Non-critical error.
                print!("WARNING: {}", AppError::new(&e.to_string()));
            }
            return server_config;
        }

        // Read settings from .ini file.
        if server_config.read_config(&config) {
            if let Err(e) = server_config.save_config() {
                // Non-critical error.
                print!("WARNING: {}", AppError::new(&e.to_string()));
            }
        }

        server_config
    }
    fn default() -> Self {
        let port_for_reporters = ConfigManager::generate_random_port(0);
        let port_for_clients = ConfigManager::generate_random_port(port_for_reporters);
        Self {
            port_for_reporters,
            port_for_clients,
            max_attachment_size_in_mb: DEFAULT_MAX_ATTACHMENT_SIZE_IN_MB,
            max_allowed_login_attempts: DEFAULT_MAX_ALLOWED_LOGIN_ATTEMPTS,
            ban_time_duration_in_min: DEFAULT_BAN_TIME_DURATION_IN_MIN,
            config_file_path: ConfigManager::get_config_file_path(),
            log_file_path: ConfigManager::get_log_file_path(),
        }
    }
    /// Saves the current configuration to a file.
    fn save_config(&self) -> Result<(), AppError> {
        let mut config = Ini::new();

        // Server section started.

        // Port for reporters.
        config.set(
            CONFIG_SERVER_SECTION_NAME,
            CONFIG_PORT_REPORTER_PARAM,
            Some(self.port_for_reporters.to_string()),
        );

        // Port for clients.
        config.set(
            CONFIG_SERVER_SECTION_NAME,
            CONFIG_PORT_CLIENT_PARAM,
            Some(self.port_for_clients.to_string()),
        );

        // Max attachment size.
        config.set(
            CONFIG_SERVER_SECTION_NAME,
            CONFIG_MAX_ATTACHMENT_SIZE_IN_MB_PARAM,
            Some(self.max_attachment_size_in_mb.to_string()),
        );

        // Login section started.

        // Max allowed login attempts until ban.
        config.set(
            CONFIG_LOGIN_SECTION_NAME,
            CONFIG_MAX_ALLOWED_LOGIN_ATTEMPTS_PARAM,
            Some(self.max_allowed_login_attempts.to_string()),
        );

        // Ban time duration.
        config.set(
            CONFIG_LOGIN_SECTION_NAME,
            CONFIG_BAN_TIME_DURATION_IN_MIN,
            Some(self.ban_time_duration_in_min.to_string()),
        );

        // Write to disk.
        if let Err(e) = config.write(&self.config_file_path) {
            return Err(AppError::new(&e.to_string()));
        }

        Ok(())
    }
    /// Read config from file.
    ///
    /// Returns `true` if some values were empty and now we are using
    /// default values for them, `false` if the file had all needed values set.
    fn read_config(&mut self, config: &Ini) -> bool {
        let mut some_values_were_empty = false;

        // Server section started.

        // Read port for reporters.
        if ConfigManager::read_value(
            config,
            CONFIG_SERVER_SECTION_NAME,
            CONFIG_PORT_REPORTER_PARAM,
            &mut self.port_for_reporters,
            ConfigManager::generate_random_port(0),
        ) {
            some_values_were_empty = true;
        }

        // Read port for clients.
        if ConfigManager::read_value(
            config,
            CONFIG_SERVER_SECTION_NAME,
            CONFIG_PORT_CLIENT_PARAM,
            &mut self.port_for_clients,
            ConfigManager::generate_random_port(self.port_for_reporters),
        ) {
            some_values_were_empty = true;
        }

        // Read max allowed attachment size.
        if ConfigManager::read_value(
            config,
            CONFIG_SERVER_SECTION_NAME,
            CONFIG_MAX_ATTACHMENT_SIZE_IN_MB_PARAM,
            &mut self.max_attachment_size_in_mb,
            DEFAULT_MAX_ATTACHMENT_SIZE_IN_MB,
        ) {
            some_values_were_empty = true;
        }

        // Login section started.

        // Read max allowed login attempts until ban.
        if ConfigManager::read_value(
            config,
            CONFIG_LOGIN_SECTION_NAME,
            CONFIG_MAX_ALLOWED_LOGIN_ATTEMPTS_PARAM,
            &mut self.max_allowed_login_attempts,
            DEFAULT_MAX_ALLOWED_LOGIN_ATTEMPTS,
        ) {
            some_values_were_empty = true;
        }

        // Read ban time duration.
        if ConfigManager::read_value(
            config,
            CONFIG_LOGIN_SECTION_NAME,
            CONFIG_BAN_TIME_DURATION_IN_MIN,
            &mut self.ban_time_duration_in_min,
            DEFAULT_BAN_TIME_DURATION_IN_MIN,
        ) {
            some_values_were_empty = true;
        }

        // New settings go here.
        // Please, don't forget to use 'some_values_were_empty'.

        some_values_were_empty
    }
    /// Reads a value from .ini file into `param` parameter.
    ///
    /// Returns `true` if the specified key does not exist
    /// or if parse failed, thus using `default_value` to assign `param`.
    /// Returns `false` if successfully read a value.
    fn read_value<T>(
        config: &Ini,
        section: &str,
        key: &str,
        param: &mut T,
        default_value: T,
    ) -> bool
    where
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
    {
        let value = config.get(section, key);
        match value {
            Some(value) => {
                let value = value.parse::<T>();
                if let Err(e) = value {
                    println!(
                    "WARNING: could not parse \"{}\" value, using default value instead (error: {}).",
                    key,
                    e
                );
                    *param = default_value;
                    true
                } else {
                    *param = value.unwrap();
                    false
                }
            }
            None => {
                *param = default_value;
                true
            }
        }
    }
    /// Returns path used to store configuration.
    fn get_config_file_path() -> PathBuf {
        #[cfg(any(windows, unix))]
        {
            let app_dirs = AppDirs::new(Some(CONFIG_FILE_DIR), true).unwrap_or_else(|| {
                panic!(
                    "An error occurred at [{}, {}]: can't read user dirs.",
                    file!(),
                    line!(),
                )
            });

            let mut config_path = app_dirs.config_dir;

            // Create directory if not exists.
            if !config_path.exists() {
                if let Err(e) = create_dir_all(&config_path) {
                    panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
                }
            }

            config_path.push(CONFIG_FILE_NAME);
            config_path
        }
        #[cfg(not(any(windows, unix)))]
        {
            compile_error!("Reporter is not implemented for this OS.");
        }
    }
    /// Returns path used to store log file.
    fn get_log_file_path() -> PathBuf {
        #[cfg(any(windows, unix))]
        {
            let app_dirs = UserDirs::new().unwrap_or_else(|| {
                panic!(
                    "An error occurred at [{}, {}]: can't read user dirs.",
                    file!(),
                    line!(),
                )
            });

            let mut log_path = app_dirs.document_dir;

            log_path.push(CONFIG_FILE_DIR);

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
    /// Generates random port value.
    fn generate_random_port(exclude_port: u16) -> u16 {
        let mut rng = rand::thread_rng();

        loop {
            let port = rng.gen_range(RANDOM_PORT_RANGE);
            if port != exclude_port {
                return port;
            }
        }
    }
}
