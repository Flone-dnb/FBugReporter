// External.
use num_bigint::{BigUint, RandomBits};
#[cfg(target_os = "windows")]
use platform_dirs::UserDirs;
use rand::Rng;

// Std.
use std::fs::*;
use std::io::prelude::*;
use std::path::Path;

// Custom.
use super::logger_service::LOG_FILE_NAME;

const CONFIG_FILE_VERSION: u32 = 0;
const CONFIG_FILE_MAGIC_NUMBER: u16 = 1919;
const CONFIG_FILE_NAME: &str = "server.config";
const PORT_RANGE: std::ops::Range<u16> = 7000..65535;
const SERVER_PASSWORD_BIT_COUNT: u64 = 1024;

#[derive(Debug)]
pub struct ServerConfig {
    pub server_port: u16,
    pub server_password: String,
    pub config_file_path: String,
    pub log_file_path: String,
}

impl ServerConfig {
    pub fn new() -> Result<Self, String> {
        let mut server_config = ServerConfig::default();

        // Get config path.
        let config_file_path = ServerConfig::get_config_file_path();

        if Path::new(&config_file_path).exists() {
            // Read existing config file.
            if let Err(msg) = server_config.read_config() {
                return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
            }
        } else {
            // Create new config file with default settings.
            if let Err(msg) = server_config.save_config() {
                return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
            }
        }

        Ok(server_config)
    }
    pub fn refresh_password(&mut self) -> Result<(), String> {
        let mut rng = rand::thread_rng();
        let server_key: BigUint = rng.sample(RandomBits::new(SERVER_PASSWORD_BIT_COUNT));

        self.server_password = server_key.to_str_radix(16);

        // Save to config.
        if let Err(msg) = self.save_config() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn refresh_port(&mut self) -> Result<(), String> {
        let mut rng = rand::thread_rng();
        self.server_port = rng.gen_range(PORT_RANGE);

        // Save to config.
        if let Err(msg) = self.save_config() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    pub fn set_port(&mut self, port: u16) -> Result<(), String> {
        self.server_port = port;

        // Save to config.
        if let Err(msg) = self.save_config() {
            return Err(format!("{} at [{}, {}]\n\n", msg, file!(), line!()));
        }

        Ok(())
    }
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let server_key: BigUint = rng.sample(RandomBits::new(SERVER_PASSWORD_BIT_COUNT));

        Self {
            server_port: rng.gen_range(PORT_RANGE),
            server_password: server_key.to_str_radix(16),
            config_file_path: ServerConfig::get_config_file_path(),
            log_file_path: ServerConfig::get_log_file_path(),
        }
    }
    fn save_config(&self) -> Result<(), String> {
        // Get config path.
        let config_file_path = ServerConfig::get_config_file_path();

        if Path::new(&config_file_path).exists() {
            // Remove existing (old) config file.
            if let Err(e) = std::fs::remove_file(&config_file_path) {
                return Err(format!(
                    "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                    file!(),
                    line!(),
                    e,
                    config_file_path,
                ));
            }
        }

        // Create new config file.
        let config_file = File::create(&config_file_path);
        if let Err(e) = config_file {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path,
            ));
        }
        let mut config_file = config_file.unwrap();

        // Write magic number.
        if let Err(e) = config_file.write(&bincode::serialize(&CONFIG_FILE_MAGIC_NUMBER).unwrap()) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path,
            ));
        }

        // Write config file version.
        let config_version = CONFIG_FILE_VERSION;
        if let Err(e) = config_file.write(&bincode::serialize(&config_version).unwrap()) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path,
            ));
        }

        // Write server port.
        if let Err(e) = config_file.write(&bincode::serialize(&self.server_port).unwrap()) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path,
            ));
        }

        // Write server password size.
        let pass_size: u32 = self.server_password.len() as u32;
        if let Err(e) = config_file.write(&bincode::serialize(&pass_size).unwrap()) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path,
            ));
        }

        // Write server password.
        if !self.server_password.is_empty() {
            if let Err(e) = config_file.write(self.server_password.as_bytes()) {
                return Err(format!(
                    "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                    file!(),
                    line!(),
                    e,
                    config_file_path,
                ));
            }
        }

        Ok(())
    }
    fn read_config(&mut self) -> Result<(), String> {
        // Get config path.
        let config_file_path = ServerConfig::get_config_file_path();

        if !Path::new(&config_file_path).exists() {
            return Err(format!(
                "An error occurred at [{}, {}]: config file does not exist (config path: {})\n\n",
                file!(),
                line!(),
                config_file_path,
            ));
        }

        // Open config file.
        let config_file = File::open(&config_file_path);
        if let Err(e) = config_file {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path
            ));
        }
        let mut config_file = config_file.unwrap();

        // Read magic number.
        let mut buf = vec![0u8; std::mem::size_of::<u16>()];
        if let Err(e) = config_file.read(&mut buf) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path
            ));
        }
        let magic_number = bincode::deserialize::<u16>(&buf).unwrap();
        if magic_number != CONFIG_FILE_MAGIC_NUMBER {
            return Err(format!(
                "An error occurred at [{}, {}]: file magic number ({}) is not equal to config magic number ({})\n\n",
                file!(),
                line!(),
                magic_number,
                CONFIG_FILE_MAGIC_NUMBER,
            ));
        }

        // Read config version.
        let mut buf = vec![0u8; std::mem::size_of::<u32>()];
        if let Err(e) = config_file.read(&mut buf) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path
            ));
        }
        // use it to handle old config versions
        let config_version = bincode::deserialize::<u32>(&buf).unwrap();

        // Read server port.
        let mut buf = vec![0u8; std::mem::size_of::<u16>()];
        if let Err(e) = config_file.read(&mut buf) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path
            ));
        }
        self.server_port = bincode::deserialize::<u16>(&buf).unwrap();

        // Read server password size.
        let mut buf = vec![0u8; std::mem::size_of::<u32>()];
        let mut _password_byte_count = 0u32;
        if let Err(e) = config_file.read(&mut buf) {
            return Err(format!(
                "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                file!(),
                line!(),
                e,
                config_file_path
            ));
        }
        _password_byte_count = bincode::deserialize::<u32>(&buf).unwrap();

        // Read server password.
        let mut buf = vec![0u8; _password_byte_count as usize];
        if _password_byte_count > 0 {
            if let Err(e) = config_file.read(&mut buf) {
                return Err(format!(
                    "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                    file!(),
                    line!(),
                    e,
                    config_file_path
                ));
            }

            let server_pass = std::str::from_utf8(&buf);
            if let Err(e) = server_pass {
                return Err(format!(
                    "An error occurred at [{}, {}]: {:?} (config path: {})\n\n",
                    file!(),
                    line!(),
                    e,
                    config_file_path
                ));
            }

            self.server_password = String::from(server_pass.unwrap());
        }

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
