// Std.
#[cfg(target_os = "windows")]
use std::fs::create_dir;
#[cfg(target_os = "windows")]
use std::path::Path;

// External.
use configparser::ini::Ini;
#[cfg(target_os = "windows")]
use platform_dirs::UserDirs;

const CONFIG_FILE_NAME: &str = "client_config.ini";
#[cfg(target_os = "windows")]
const CONFIG_FILE_DIR: &str = "FBugReporter";

const CONFIG_SECTION_NAME: &str = "client";
const CONFIG_SERVER_PARAM: &str = "server";
const CONFIG_PORT_PARAM: &str = "port";
const CONFIG_USERNAME_PARAM: &str = "username";

pub struct ConfigService {
    pub server: String,
    pub port: String,
    pub username: String,
}

impl ConfigService {
    /// Attempts to read existing config file.
    /// Otherwise, returns an empty configuration.
    pub fn new() -> Self {
        let mut config = ConfigService::default();

        let mut config_file = Ini::new();
        let map = config_file.load(ConfigService::get_config_file_path());

        if map.is_err() {
            return config;
        }

        config.read_config_file(&mut config_file);

        config
    }
    pub fn write_config_to_file(&self) {
        let mut config_file = Ini::new();

        config_file.setstr(CONFIG_SECTION_NAME, CONFIG_SERVER_PARAM, Some(&self.server));
        config_file.setstr(CONFIG_SECTION_NAME, CONFIG_PORT_PARAM, Some(&self.port));
        config_file.setstr(
            CONFIG_SECTION_NAME,
            CONFIG_USERNAME_PARAM,
            Some(&self.username),
        );

        let config_path = ConfigService::get_config_file_path();
        if let Err(e) = config_file.write(&config_path) {
            println!(
                "WARNING: failed to save configuration to the file \"{}\" (error: {}).",
                &config_path,
                e.to_string()
            );
        }
    }
    pub fn get_config_file_path() -> String {
        #[cfg(target_os = "linux")]
        {
            let mut config_path = String::from(std::env::current_dir().unwrap().to_str().unwrap());

            // Check ending.
            if !config_path.ends_with('/') {
                config_path += "/";
            }

            return config_path + CONFIG_FILE_NAME;
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
            let mut config_path = String::from(user_dirs.document_dir.to_str().unwrap());

            // Check ending.
            if !config_path.ends_with('\\') {
                config_path += "\\";
            }

            config_path += CONFIG_FILE_DIR;
            config_path += "\\";

            // Create directory if not exists.
            if !Path::new(&config_path).exists() {
                if let Err(e) = create_dir(&config_path) {
                    panic!("An error occurred at [{}, {}]: {:?}", file!(), line!(), e);
                }
            }

            config_path += CONFIG_FILE_NAME;
            return config_path;
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            compile_error!("Client is not implemented for this OS.");
        }
    }
    fn read_config_file(&mut self, config: &Ini) {
        // Read server.
        let server = config.get(CONFIG_SECTION_NAME, CONFIG_SERVER_PARAM);
        if server.is_some() {
            self.server = server.unwrap();
        }

        // Read port.
        let port = config.get(CONFIG_SECTION_NAME, CONFIG_PORT_PARAM);
        if port.is_some() {
            self.port = port.unwrap();
        }

        // Read username.
        let username = config.get(CONFIG_SECTION_NAME, CONFIG_USERNAME_PARAM);
        if username.is_some() {
            self.username = username.unwrap();
        }
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self {
            server: String::new(),
            port: String::new(),
            username: String::new(),
        }
    }
}
