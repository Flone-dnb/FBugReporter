// External.
use configparser::ini::Ini;
use druid::widget::prelude::*;
use druid::{Color, Lens};

const CONFIG_THEME_SECTION_NAME: &str = "theme";

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
        // Try reading theme from .ini file.
        let mut config = Ini::new();
        let map = config.load("theme.ini");
        if let Err(e) = map {
            println!("{:?}\nUsing default theme.", e);

            return Self::default();
        }

        let mut theme = Self::default();

        read_theme_color_hex("background_color", &mut theme.background_color, &config);
        read_theme_color_hex("placeholder_color", &mut theme.placeholder_color, &config);
        read_theme_color_hex(
            "textbox_background_color",
            &mut theme.textbox_background_color,
            &config,
        );
        read_theme_color_hex(
            "inactive_border_color",
            &mut theme.inactive_border_color,
            &config,
        );
        read_theme_color_hex(
            "active_border_color",
            &mut theme.active_border_color,
            &config,
        );
        read_theme_color_hex(
            "text_selection_color",
            &mut theme.text_selection_color,
            &config,
        );
        read_theme_color_hex("button_dark_color", &mut theme.button_dark_color, &config);
        read_theme_color_hex("button_light_color", &mut theme.button_light_color, &config);
        read_theme_param_float("border_radius", &mut theme.border_radius, &config);

        theme
    }
}

impl Default for ApplicationTheme {
    fn default() -> Self {
        ApplicationTheme {
            background_color: Color::rgb8(30, 26, 22),
            placeholder_color: Color::rgb8(65, 60, 55),
            textbox_background_color: Color::rgb8(35, 30, 25),
            inactive_border_color: Color::rgba(0.0, 0.0, 0.0, 0.0),
            active_border_color: Color::rgb8(181, 98, 2),
            text_selection_color: Color::rgb8(181, 98, 2),
            button_dark_color: Color::rgb8(181, 98, 2),
            button_light_color: Color::rgb8(181, 98, 2),
            border_radius: 10.0,
        }
    }
}

fn read_theme_color_hex(param: &str, color: &mut Color, config: &Ini) {
    let value = config.get(CONFIG_THEME_SECTION_NAME, param);
    if value.is_some() {
        let value = value.unwrap();
        let new_color = Color::from_hex_str(&value);
        if let Err(ref e) = new_color {
            println!(
                "ERROR: could not parse value of theme parameter '{}', error: {}.",
                param, e
            );
        } else {
            *color = new_color.unwrap();
        }
    }
}

fn read_theme_param_float(param: &str, input: &mut f64, config: &Ini) {
    let value = config.getfloat(CONFIG_THEME_SECTION_NAME, param);
    if let Err(e) = value {
        println!(
            "ERROR: could not parse value of theme parameter '{}', error: {}.",
            param, e
        );
        return;
    }
    let value = value.unwrap();
    if value.is_some() {
        *input = value.unwrap();
    } else {
        println!("ERROR: param '{}' is None.", param);
    }
}
