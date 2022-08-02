// Std.
use std::fs::create_dir_all;
use std::path::PathBuf;

// External.
use configparser::ini::Ini;
use druid::widget::prelude::*;
use druid::{Color, Lens};
use platform_dirs::AppDirs;

// Custom.
use shared::misc::error::AppError;

const CONFIG_THEME_DIR_NAME: &str = "FBugReporter";
const CONFIG_THEME_FILE_NAME: &str = "client_theme.ini";
const CONFIG_THEME_SECTION_NAME: &str = "theme";
const CONFIG_THEME_BACKGROUND_COLOR_PARAM: &str = "background_color";
const CONFIG_THEME_PLACEHOLDER_COLOR_PARAM: &str = "placeholder_color";
const CONFIG_THEME_TEXTBOX_BACKGROUND_COLOR_PARAM: &str = "textbox_background_color";
const CONFIG_THEME_TEXT_SELECTION_COLOR_PARAM: &str = "text_selection_color";
const CONFIG_THEME_ACTIVE_BORDER_COLOR_PARAM: &str = "active_border_color";
const CONFIG_THEME_INACTIVE_BORDER_COLOR_PARAM: &str = "inactive_border_color";
const CONFIG_THEME_BUTTON_DARK_COLOR_PARAM: &str = "button_dark_color";
const CONFIG_THEME_BUTTON_LIGHT_COLOR_PARAM: &str = "button_light_color";
const CONFIG_THEME_BORDER_RADIUS_PARAM: &str = "border_radius";

#[derive(Clone, Data, Lens, Debug)]
pub struct ApplicationTheme {
    pub background_color: Color,
    pub placeholder_color: Color,
    pub textbox_background_color: Color,
    pub text_selection_color: Color,
    pub active_border_color: Color,
    pub inactive_border_color: Color,
    pub button_dark_color: Color,
    pub button_light_color: Color,
    pub border_radius: f64,
}

impl ApplicationTheme {
    pub fn new() -> Self {
        let mut theme = ApplicationTheme::default();

        // Try reading theme from .ini file.
        let mut config = Ini::new();
        let theme_config_path = Self::get_theme_config_file_path();
        let map = config.load(&theme_config_path);
        if map.is_err() {
            println!(
                "INFO: could not open the theme file \"{0}\", using default values \
                and creating a new \"{0}\" theme file at \"{1}\".",
                CONFIG_THEME_FILE_NAME,
                theme_config_path.to_string_lossy()
            );
            // No file found, create a new file.
            if let Err(e) = theme.save_theme() {
                // Non-critical error.
                print!("WARNING: {}", AppError::new(&e.to_string()));
            }
            return theme;
        }

        let mut some_values_were_empty = false;
        if read_theme_color_hex(
            CONFIG_THEME_BACKGROUND_COLOR_PARAM,
            &mut theme.background_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_PLACEHOLDER_COLOR_PARAM,
            &mut theme.placeholder_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_TEXTBOX_BACKGROUND_COLOR_PARAM,
            &mut theme.textbox_background_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_INACTIVE_BORDER_COLOR_PARAM,
            &mut theme.inactive_border_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_ACTIVE_BORDER_COLOR_PARAM,
            &mut theme.active_border_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_TEXT_SELECTION_COLOR_PARAM,
            &mut theme.text_selection_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_BUTTON_DARK_COLOR_PARAM,
            &mut theme.button_dark_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_color_hex(
            CONFIG_THEME_BUTTON_LIGHT_COLOR_PARAM,
            &mut theme.button_light_color,
            &config,
        ) {
            some_values_were_empty = true;
        }
        if read_theme_param_float(
            CONFIG_THEME_BORDER_RADIUS_PARAM,
            &mut theme.border_radius,
            &config,
        ) {
            some_values_were_empty = true;
        }

        if some_values_were_empty {
            // Create a new file with all values filled.
            if let Err(e) = theme.save_theme() {
                // Non-critical error.
                print!("WARNING: {}", AppError::new(&e.to_string()));
            }
        }

        theme
    }
    fn save_theme(&self) -> Result<(), AppError> {
        let mut config = Ini::new();

        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_BACKGROUND_COLOR_PARAM,
            Some(&format!("{:?}", self.background_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_PLACEHOLDER_COLOR_PARAM,
            Some(&format!("{:?}", self.placeholder_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_TEXTBOX_BACKGROUND_COLOR_PARAM,
            Some(&format!("{:?}", self.textbox_background_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_INACTIVE_BORDER_COLOR_PARAM,
            Some(&format!("{:?}", self.inactive_border_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_ACTIVE_BORDER_COLOR_PARAM,
            Some(&format!("{:?}", self.active_border_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_TEXT_SELECTION_COLOR_PARAM,
            Some(&format!("{:?}", self.text_selection_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_BUTTON_DARK_COLOR_PARAM,
            Some(&format!("{:?}", self.button_dark_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_BUTTON_LIGHT_COLOR_PARAM,
            Some(&format!("{:?}", self.button_light_color)[1..]),
        );
        config.setstr(
            CONFIG_THEME_SECTION_NAME,
            CONFIG_THEME_BORDER_RADIUS_PARAM,
            Some(&self.border_radius.to_string()),
        );

        if let Err(e) = config.write(Self::get_theme_config_file_path()) {
            return Err(AppError::new(&e.to_string()));
        }

        Ok(())
    }
    pub fn get_theme_config_file_path() -> PathBuf {
        #[cfg(any(windows, unix))]
        {
            let app_dirs = AppDirs::new(Some(CONFIG_THEME_DIR_NAME), true).unwrap_or_else(|| {
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

            config_path.push(CONFIG_THEME_FILE_NAME);
            config_path
        }
        #[cfg(not(any(windows, unix)))]
        {
            compile_error!("Client is not implemented for this OS.");
        }
    }
}

impl Default for ApplicationTheme {
    fn default() -> Self {
        ApplicationTheme {
            background_color: Color::rgb8(30, 26, 22),
            placeholder_color: Color::rgb8(65, 60, 55),
            textbox_background_color: Color::rgb8(35, 30, 25),
            inactive_border_color: Color::rgb8(181, 98, 2),
            active_border_color: Color::rgb8(181, 98, 2),
            text_selection_color: Color::rgb8(181, 98, 2),
            button_dark_color: Color::rgb8(181, 98, 2),
            button_light_color: Color::rgb8(181, 98, 2),
            border_radius: 10.0,
        }
    }
}

/// Read color parameter from hex string.
///
/// Returns `true` if the value was empty, `false` if it was set.
fn read_theme_color_hex(param: &str, color: &mut Color, config: &Ini) -> bool {
    let mut value_was_empty = true;

    let value = config.get(CONFIG_THEME_SECTION_NAME, param);
    if let Some(value) = value {
        if !value.is_empty() {
            let new_color = Color::from_hex_str(&value);
            if let Err(ref e) = new_color {
                println!(
                    "ERROR: could not parse value of theme parameter '{}', error: {}.",
                    param, e
                );
            } else {
                *color = new_color.unwrap();
                value_was_empty = false;
            }
        }
    }

    value_was_empty
}

/// Read float parameter from string.
///
/// Returns `true` if the value was empty, `false` if it was set.
fn read_theme_param_float(param: &str, input: &mut f64, config: &Ini) -> bool {
    let mut value_was_empty = true;

    let value = config.getfloat(CONFIG_THEME_SECTION_NAME, param);
    if let Err(e) = value {
        println!(
            "ERROR: could not parse value of theme parameter '{}', error: {}.",
            param, e
        );
    } else {
        let value = value.unwrap();
        match value {
            Some(value) => {
                *input = value;
                value_was_empty = false;
            }
            None => {
                println!("ERROR: param '{}' is None.", param);
            }
        }
    }

    value_was_empty
}
