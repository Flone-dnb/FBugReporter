// External.
#[cfg(target_os = "windows")]
use platform_dirs::UserDirs;
use rand::Rng;

// Std.
use std::fs::*;
use std::io::prelude::*;
use std::path::Path;

// Custom.
use super::logger_service::LOG_FILE_NAME;
use crate::error::AppError;

const CONFIG_FILE_VERSION: u32 = 0;
const CONFIG_FILE_MAGIC_NUMBER: u16 = 1919;
const CONFIG_FILE_NAME: &str = "server.config";
const PORT_RANGE: std::ops::Range<u16> = 7000..65535;

#[derive(Debug)]
pub struct ServerConfig {
    pub server_port: u16,
    pub config_file_path: String,
    pub log_file_path: String,
}

impl ServerConfig {
    pub fn new() -> Result<Self, AppError> {
        let mut server_config = ServerConfig::default();

        // Get config path.
        let config_file_path = ServerConfig::get_config_file_path();

        if Path::new(&config_file_path).exists() {
            // Read existing config file.
            if let Err(err) = server_config.read_config() {
                return Err(err.add_entry(file!(), line!()));
            }
        } else {
            // Create new config file with default settings.
            if let Err(err) = server_config.save_config() {
                return Err(err.add_entry(file!(), line!()));
            }
        }

        Ok(server_config)
    }
    pub fn refresh_port(&mut self) -> Result<(), AppError> {
        let mut rng = rand::thread_rng();
        self.server_port = rng.gen_range(PORT_RANGE);

        // Save to config.
        if let Err(err) = self.save_config() {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(())
    }
    pub fn set_port(&mut self, port: u16) -> Result<(), AppError> {
        self.server_port = port;

        // Save to config.
        if let Err(err) = self.save_config() {
            return Err(err.add_entry(file!(), line!()));
        }

        Ok(())
    }
    fn default() -> Self {
        let mut rng = rand::thread_rng();

        Self {
            server_port: rng.gen_range(PORT_RANGE),
            config_file_path: ServerConfig::get_config_file_path(),
            log_file_path: ServerConfig::get_log_file_path(),
        }
    }
    fn save_config(&self) -> Result<(), AppError> {
        // Get config path.
        let config_file_path = ServerConfig::get_config_file_path();

        if Path::new(&config_file_path).exists() {
            // Remove existing (old) config file.
            if let Err(e) = std::fs::remove_file(&config_file_path) {
                return Err(AppError::new(
                    &format!("{:?} (config path: {})", e, config_file_path),
                    file!(),
                    line!(),
                ));
            }
        }

        // Create new config file.
        let config_file = File::create(&config_file_path);
        if let Err(e) = config_file {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }
        let mut config_file = config_file.unwrap();

        // Write magic number.
        if let Err(e) = config_file.write(&bincode::serialize(&CONFIG_FILE_MAGIC_NUMBER).unwrap()) {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }

        // Write config file version.
        let config_version = CONFIG_FILE_VERSION;
        if let Err(e) = config_file.write(&bincode::serialize(&config_version).unwrap()) {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }

        // Write server port.
        if let Err(e) = config_file.write(&bincode::serialize(&self.server_port).unwrap()) {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }

        Ok(())
    }
    fn read_config(&mut self) -> Result<(), AppError> {
        // Get config path.
        let config_file_path = ServerConfig::get_config_file_path();

        if !Path::new(&config_file_path).exists() {
            return Err(AppError::new(
                &format!(
                    "config file does not exist (config path: {})",
                    config_file_path
                ),
                file!(),
                line!(),
            ));
        }

        // Open config file.
        let config_file = File::open(&config_file_path);
        if let Err(e) = config_file {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }
        let mut config_file = config_file.unwrap();

        // Read magic number.
        let mut buf = vec![0u8; std::mem::size_of::<u16>()];
        if let Err(e) = config_file.read(&mut buf) {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }
        let magic_number = bincode::deserialize::<u16>(&buf).unwrap();
        if magic_number != CONFIG_FILE_MAGIC_NUMBER {
            return Err(AppError::new(
                &format!(
                    "file magic number ({}) is not equal to config magic number ({})",
                    magic_number, CONFIG_FILE_MAGIC_NUMBER,
                ),
                file!(),
                line!(),
            ));
        }

        // Read config version.
        let mut buf = vec![0u8; std::mem::size_of::<u32>()];
        if let Err(e) = config_file.read(&mut buf) {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }
        // use it to handle old config versions
        let config_version = bincode::deserialize::<u32>(&buf).unwrap();

        // Read server port.
        let mut buf = vec![0u8; std::mem::size_of::<u16>()];
        if let Err(e) = config_file.read(&mut buf) {
            return Err(AppError::new(
                &format!("{:?} (config path: {})", e, config_file_path),
                file!(),
                line!(),
            ));
        }
        self.server_port = bincode::deserialize::<u16>(&buf).unwrap();

        // ---------------------------------------------------------------------
        //
        // please use 'config_version' variable to handle old config versions...
        //
        // ---------------------------------------------------------------------

        Ok(())
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
}
