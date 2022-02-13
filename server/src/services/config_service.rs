// Std.
use std::ops::Range;

// External.
use configparser::ini::Ini;
#[cfg(target_os = "windows")]
use platform_dirs::UserDirs;
use rand::Rng;

// Custom.
use super::logger_service::LOG_FILE_NAME;
use crate::error::AppError;

const RANDOM_PORT_RANGE: Range<u16> = 7000..65535;
const CONFIG_FILE_NAME: &str = "server_config.ini";
const CONFIG_SECTION_NAME: &str = "server";
const CONFIG_PORT_PARAM: &str = "server_port";

#[derive(Debug)]
pub struct ServerConfig {
    pub server_port: u16,
    pub config_file_path: String,
    pub log_file_path: String,
}

impl ServerConfig {
    /// Reads config values from .ini file if it exists,
    /// otherwise using default values and creating a new config .ini file.
    pub fn new() -> Self {
        let mut server_config = ServerConfig::default();

        // Try reading config from .ini file.
        let mut config = Ini::new();
        let map = config.load(CONFIG_FILE_NAME);
        if map.is_err() {
            println!(
                "INFO: could not open the config file \"{0}\", using default values \
                and creating a new \"{0}\" configuration file.",
                CONFIG_FILE_NAME
            );
            // No file found, create a new file.
            if let Err(e) = server_config.save_config() {
                // Non-critical error.
                print!(
                    "WARNING: {}",
                    AppError::new(&e.to_string(), file!(), line!())
                );
            }
            return server_config;
        }

        // Read settings from .ini file.
        if server_config.read_config(&config) == true {
            if let Err(e) = server_config.save_config() {
                // Non-critical error.
                print!(
                    "WARNING: {}",
                    AppError::new(&e.to_string(), file!(), line!())
                );
            }
        }

        server_config
    }
    fn default() -> Self {
        Self {
            server_port: ServerConfig::generate_random_port(),
            config_file_path: ServerConfig::get_config_file_path(),
            log_file_path: ServerConfig::get_log_file_path(),
        }
    }
    fn save_config(&self) -> Result<(), AppError> {
        let mut config = Ini::new();

        config.set(
            CONFIG_SECTION_NAME,
            CONFIG_PORT_PARAM,
            Some(self.server_port.to_string()),
        );

        // Write to disk.
        if let Err(e) = config.write(CONFIG_FILE_NAME) {
            return Err(AppError::new(&e.to_string(), file!(), line!()));
        }

        Ok(())
    }
    /// Read config from file.
    ///
    /// Returns `true` if some values were empty and now we are using
    /// default values for them, `false` if the file had all needed values set.
    fn read_config(&mut self, config: &Ini) -> bool {
        let mut some_values_were_empty = false;

        // Read port.
        let port = config.get(CONFIG_SECTION_NAME, CONFIG_PORT_PARAM);
        if port.is_none() {
            self.server_port = ServerConfig::generate_random_port();
            some_values_were_empty = true;
        } else {
            let port = port.unwrap().parse::<u16>();
            if let Err(e) = port {
                println!(
                    "WARNING: could not parse \"{}\" value, using random port instead (error: {}).",
                    CONFIG_PORT_PARAM,
                    e.to_string()
                );
                self.server_port = ServerConfig::generate_random_port();
                some_values_were_empty = true;
            } else {
                self.server_port = port.unwrap();
            }
        }

        // New settings go here.
        // Please, don't forget to use 'some_values_were_empty'.

        some_values_were_empty
    }
    fn get_config_file_path() -> String {
        let mut config_path = String::from(std::env::current_dir().unwrap().to_str().unwrap());

        // Check ending.
        #[cfg(target_os = "linux")]
        {
            if !config_path.ends_with('/') {
                config_path += "/";
            }
        }
        #[cfg(target_os = "windows")]
        {
            if !config_path.ends_with('\\') {
                config_path += "\\";
            }
        }

        config_path + CONFIG_FILE_NAME
    }

    fn get_log_file_path() -> String {
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

        log_path + LOG_FILE_NAME
    }
    fn generate_random_port() -> u16 {
        let mut rng = rand::thread_rng();
        rng.gen_range(RANDOM_PORT_RANGE)
    }
}
