// External.
#[cfg(target_os = "windows")]
use platform_dirs::UserDirs;

// Std.
use std::fs::*;
use std::io::prelude::*;
use std::path::Path;

// Custom.
use crate::global_params::*;

#[derive(Debug)]
pub struct ServerConfig {
    pub server_port: u16,
    pub server_password: String,
    pub config_file_path: String,
    pub log_file_path: String,
}

impl ServerConfig {
    pub fn new() -> Result<Self, String> {
        let config_file_path = ServerConfig::get_config_file_path();
        if let Err(msg) = config_file_path {
            return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
        }
        let config_file_path = config_file_path.unwrap();

        let mut server_config = ServerConfig::default();

        if Path::new(&config_file_path).exists() {
            // Read existing config file.
            if let Err(msg) = server_config.read_config() {
                return Err(format!("{}, at [{}, {}]", msg, file!(), line!()));
            }
        } else {
            // Create new config file with default settings.
            if let Err(msg) = server_config.save_config() {
                return Err(format!("{} at [{}, {}]", msg, file!(), line!()));
            }
        }

        TODO: add log file path
        server_config.config_file_path = config_file_path.clone();
        Ok(server_config)
    }
    fn default() -> Self {
        Self {
            server_port: SERVER_DEFAULT_PORT,
            server_password: String::from(""),
            config_file_path: String::from(""),
            log_file_path: String::from(""),
        }
    }
    fn get_config_file_path() -> Result<String, String> {
        let res = ServerConfig::get_config_file_dir();
        match res {
            Ok(path) => Ok(path + CONFIG_FILE_NAME),
            Err(msg) => Err(format!("{} at [{}, {}]", msg, file!(), line!())),
        }
    }

    fn get_config_file_dir() -> Result<String, String> {
        let mut _config_dir = String::new();
        #[cfg(target_os = "windows")]
        {
            let user_dirs = UserDirs::new();
            if user_dirs.is_none() {
                return Err(format!(
                    "UserDirs::new() failed, error: can't read user dirs at [{}, {}]",
                    file!(),
                    line!(),
                ));
            }
            let user_dirs = user_dirs.unwrap();
            _config_dir = String::from(user_dirs.document_dir.to_str().unwrap());
        }

        #[cfg(target_os = "linux")]
        {
            _config_dir = format!(
                "/home/{}/.config",
                users::get_current_username().unwrap().to_str().unwrap()
            );
            if !Path::new(&_config_dir).exists() {
                if let Err(e) = create_dir(&_config_dir) {
                    panic!(
                        "unable to create a .config directory ({}): {}",
                        &_config_dir, e
                    );
                }
            }
        }

        #[cfg(target_os = "windows")]
        if !_config_dir.ends_with('\\') {
            _config_dir += "\\";
        }

        #[cfg(target_os = "linux")]
        if !_config_dir.ends_with('/') {
            _config_dir += "/";
        }

        _config_dir += CONFIG_DIR_NAME;
        if !Path::new(&_config_dir).exists() {
            if let Err(e) = create_dir(&_config_dir) {
                panic!(
                    "unable to create a config directory ({}), error: {}",
                    &_config_dir, e
                );
            }
        }

        #[cfg(target_os = "windows")]
        if !_config_dir.ends_with('\\') {
            _config_dir += "\\";
        }

        #[cfg(target_os = "linux")]
        if !_config_dir.ends_with('/') {
            _config_dir += "/";
        }

        Ok(_config_dir)
    }
}
