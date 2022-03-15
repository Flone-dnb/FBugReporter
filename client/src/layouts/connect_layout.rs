// External.
use druid::widget::{prelude::*, SizedBox};
use druid::widget::{Button, Flex, Label, LineBreaking, MainAxisAlignment, TextBox};
use druid::{
    piet::{ImageBuf, ImageFormat, InterpolationMode},
    widget::{FillStrat, Image},
};
use druid::{Lens, LensExt, TextAlignment, WidgetExt};

// Custom.
use crate::services::{config_service::ConfigService, net_service::ConnectResult};
use crate::{ApplicationState, Layout};

// Layout customization.
const WIDTH_PADDING: f64 = 0.25;
const LEFT_SIDE_SIZE: f64 = 0.5;
const RIGHT_SIDE_SIZE: f64 = 1.0;
const TOP_PADDING: f64 = 0.5;
const BOTTOM_PADDING: f64 = 0.75;
const ROW_SPACING: f64 = 0.25;
const BUTTONS_WIDTH_PADDING: f64 = 1.0;
const BUTTON_HEIGHT: f64 = 0.3;
const TEXT_SIZE: f64 = 20.0;

#[derive(Clone, Data, Lens)]
pub struct ConnectLayout {
    pub server: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub connect_error: String,
}

impl ConnectLayout {
    pub fn new() -> Self {
        let config_file = ConfigService::new();

        Self {
            server: config_file.server,
            port: config_file.port,
            username: config_file.username,
            password: String::new(),
            connect_error: String::new(),
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(SizedBox::empty().expand(), TOP_PADDING)
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), WIDTH_PADDING)
                    .with_flex_child(
                        Flex::column()
                            .with_flex_child(
                                Label::new("Server:").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                Label::new("Port:").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                Label::new("Username:").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                Label::new("Password:").with_text_size(TEXT_SIZE).expand(),
                                1.0,
                            ),
                        LEFT_SIDE_SIZE,
                    )
                    .with_default_spacer()
                    .with_flex_child(
                        Flex::column()
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("Server's address or a domain name...")
                                    .lens(
                                        ApplicationState::connect_layout
                                            .then(ConnectLayout::server),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("Server's port...")
                                    .lens(
                                        ApplicationState::connect_layout.then(ConnectLayout::port),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("Your username...")
                                    .lens(
                                        ApplicationState::connect_layout
                                            .then(ConnectLayout::username),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("Your password...")
                                    .lens(
                                        ApplicationState::connect_layout
                                            .then(ConnectLayout::password),
                                    )
                                    .expand(),
                                1.0,
                            ),
                        RIGHT_SIDE_SIZE,
                    )
                    .with_flex_child(SizedBox::empty().expand(), WIDTH_PADDING),
                1.0,
            )
            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    data.connect_layout.connect_error.clone()
                })
                .with_text_size(TEXT_SIZE)
                .with_text_alignment(TextAlignment::Center)
                .with_line_break_mode(LineBreaking::WordWrap),
            )
            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING)
                    .with_flex_child(
                        Button::from_label(Label::new("Connect").with_text_size(TEXT_SIZE))
                            .on_click(ConnectLayout::on_connect_clicked)
                            .expand(),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING),
                BUTTON_HEIGHT,
            )
            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING)
                    .with_flex_child(
                        Button::from_label(Label::new("About").with_text_size(TEXT_SIZE))
                            .on_click(ConnectLayout::on_settings_clicked)
                            .expand(),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING),
                BUTTON_HEIGHT,
            )
            .with_flex_child(SizedBox::empty().expand(), BOTTOM_PADDING)
    }
    pub fn on_connect_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // Check if all essential fields are filled.
        if data.connect_layout.server.is_empty()
            || data.connect_layout.port.is_empty()
            || data.connect_layout.password.is_empty()
        {
            data.connect_layout.connect_error = String::from("All fields must be filled.");
            return;
        }

        // Try to parse the port string.
        let port = data.connect_layout.port.parse::<u16>();
        if port.is_err() {
            data.connect_layout.port = String::new();
            data.connect_layout.connect_error = String::from("Could not parse port value.");
            return;
        }
        let port = port.unwrap();

        // Try to connect.
        let result = data.net_service.lock().unwrap().connect(
            data.connect_layout.server.clone(),
            port,
            data.connect_layout.username.clone(),
            data.connect_layout.password.clone(),
            None,
        );
        match result {
            ConnectResult::InternalError(app_error) => {
                println!("{}", app_error);
                data.logger_service
                    .lock()
                    .unwrap()
                    .log(&app_error.to_string());
                data.connect_layout.connect_error = app_error.to_string();
            }
            ConnectResult::ConnectFailed(reason) => {
                println!("{}", reason);
                data.logger_service.lock().unwrap().log(&reason);
                data.connect_layout.connect_error = reason;
            }
            ConnectResult::NeedFirstPassword => {
                data.current_layout = Layout::ChangePassword;
            }
            ConnectResult::Connected => {
                data.connect_layout.password = String::new();
                data.current_layout = Layout::Main;
            }
        }
    }
    fn on_settings_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.current_layout = Layout::Settings;
    }
}
