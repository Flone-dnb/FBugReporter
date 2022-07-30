// Std.
use std::fs::create_dir_all;
use std::path::PathBuf;

// External.
use configparser::ini::Ini;
use platform_dirs::AppDirs;

const CONFIG_FILE_NAME: &str = "client_config.ini";
const CONFIG_FILE_DIR: &str = "FBugReporter";

const CONFIG_SECTION_NAME: &str = "client";
const CONFIG_SERVER_PARAM: &str = "server";
const CONFIG_PORT_PARAM: &str = "port";
const CONFIG_USERNAME_PARAM: &str = "username";

#[derive(Default)]
pub struct ConfigManager {
    pub server: String,
    pub port: String,
    pub username: String,
}

impl ConfigManager {
    /// Attempts to read existing config file.
    /// Otherwise, returns an empty configuration.
    pub fn new() -> Self {
        let mut config = ConfigManager::default();

        let mut config_file = Ini::new();
        let map = config_file.load(ConfigManager::get_config_file_path());

        if map.is_err() {
            return config;
        }

        config.read_config_file(&config_file);

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

        let config_path = ConfigManager::get_config_file_path();
        if let Err(e) = config_file.write(&config_path) {
            println!(
                "WARNING: failed to save configuration to the file \"{}\" (error: {}).",
                &config_path.to_string_lossy(),
                e
            );
        }
    }
    pub fn get_config_file_path() -> PathBuf {
        #[cfg(any(windows, unix))]
        {
            let app_dirs = AppDirs::new(Some(CONFIG_FILE_DIR), true).expect(&format!(
                "An error occurred at [{}, {}]: can't read user dirs.",
                file!(),
                line!(),
            ));

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
    fn read_config_file(&mut self, config: &Ini) {
        // Read server.
        let server = config.get(CONFIG_SECTION_NAME, CONFIG_SERVER_PARAM);
        if let Some(server) = server {
            self.server = server;
        }

        // Read port.
        let port = config.get(CONFIG_SECTION_NAME, CONFIG_PORT_PARAM);
        if let Some(port) = port {
            self.port = port;
        }

        // Read username.
        let username = config.get(CONFIG_SECTION_NAME, CONFIG_USERNAME_PARAM);
        if let Some(username) = username {
            self.username = username;
        }
    }
}
