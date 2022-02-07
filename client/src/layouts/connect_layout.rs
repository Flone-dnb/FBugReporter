// External.
use druid::widget::{prelude::*, SizedBox};
use druid::widget::{Button, Flex, Label, MainAxisAlignment, TextBox};
use druid::{Lens, LensExt, WidgetExt};

// Custom.
use crate::{ApplicationState, Layout};

// Layout customization.
const WIDTH_PADDING: f64 = 0.25;
const LEFT_SIDE_SIZE: f64 = 0.5;
const RIGHT_SIDE_SIZE: f64 = 1.0;
const TOP_PADDING: f64 = 0.5;
const BOTTOM_PADDING: f64 = 0.75;
const ROW_SPACING: f64 = 0.5;
const BUTTONS_WIDTH_PADDING: f64 = 1.0;
const BUTTON_HEIGHT: f64 = 0.3;
const TEXT_SIZE: f64 = 20.0;

#[derive(Clone, Data, Lens)]
pub struct ConnectLayout {
    pub server: String,
    pub port: String,
    pub password: String,
}

impl ConnectLayout {
    pub fn new() -> Self {
        Self {
            server: String::new(),
            port: String::new(),
            password: String::new(),
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
    fn on_connect_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        let port = data.connect_layout.port.parse::<u16>();
        if port.is_err() {
            data.connect_layout.port = String::new();
            // TODO: add to screen.
            return;
        }
        let port = port.unwrap();

        let result = data.net_service.lock().unwrap().connect(
            data.connect_layout.server.clone(),
            port,
            data.connect_layout.password.clone(),
        );
        if let Err(app_error) = result {
            println!("{}", app_error);
            data.logger_service
                .lock()
                .unwrap()
                .log(&app_error.to_string());
            // TODO: add to screen.
        } else {
            data.current_layout = Layout::Main;
        }
    }
    fn on_settings_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.current_layout = Layout::Settings;
    }
}
