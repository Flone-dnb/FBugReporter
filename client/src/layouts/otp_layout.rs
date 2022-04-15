// External.
use druid::widget::{prelude::*, LineBreaking, SizedBox};
use druid::widget::{Button, Flex, Label, MainAxisAlignment, TextBox};
use druid::{
    piet::{ImageBuf, ImageFormat, InterpolationMode},
    widget::{FillStrat, Image},
};
use druid::{Lens, LensExt, TextAlignment, WidgetExt};

// Custom.
use crate::services::net_service::ConnectResult;
use crate::{ApplicationState, Layout};

// Layout customization.
const WIDTH_PADDING: f64 = 0.25;
const TOP_PADDING: f64 = 0.1;
const BOTTOM_PADDING: f64 = 0.1;
const BUTTONS_WIDTH_PADDING: f64 = 1.0;
const BUTTON_HEIGHT: f64 = 0.14;
const TEXT_SIZE: f64 = 20.0;
const ROW_SPACING: f64 = 0.2;

#[derive(Clone, Data, Lens)]
pub struct OtpLayout {
    pub qr_code: Option<String>,
    otp: String,
    connect_error: String,
}

impl OtpLayout {
    pub fn new() -> Self {
        Self {
            otp: String::new(),
            qr_code: None,
            connect_error: String::new(),
        }
    }
    pub fn build_ui(&self) -> impl Widget<ApplicationState> {
        let mut qr_code_item = Flex::column();
        if self.qr_code.is_some() {
            let image = photon_rs::base64_to_image(self.qr_code.as_ref().unwrap());
            let pixels = image.get_raw_pixels();
            let image_data = ImageBuf::from_raw(
                pixels,
                ImageFormat::RgbaSeparate,
                image.get_width() as usize,
                image.get_width() as usize,
            );

            let image_widget = Image::new(image_data)
                .fill_mode(FillStrat::Fill)
                .interpolation_mode(InterpolationMode::Bilinear)
                .fix_size(
                    (image.get_width() / 4) as f64,
                    (image.get_width() / 4) as f64,
                );

            qr_code_item = qr_code_item
                .with_flex_child(
                    Flex::row()
                        .with_flex_child(SizedBox::empty().expand(), WIDTH_PADDING)
                        .with_flex_child(
                            Label::new(
                                "Use an app to scan this QR code (for example: \
                                        Google Authenticator) and \
                                        enter current OTP below.",
                            )
                            .with_text_size(TEXT_SIZE)
                            .with_text_alignment(TextAlignment::Center)
                            .with_line_break_mode(LineBreaking::WordWrap)
                            .expand(),
                            1.0,
                        )
                        .with_flex_child(SizedBox::empty().expand(), WIDTH_PADDING),
                    1.0,
                )
                .with_default_spacer()
                .with_child(image_widget);
        }

        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(SizedBox::empty().expand(), TOP_PADDING)
            .with_flex_child(qr_code_item, 1.0)
            .with_default_spacer()
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING)
                    .with_flex_child(
                        Label::new("Enter your OTP:")
                            .with_text_size(TEXT_SIZE)
                            .expand(),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING),
                BUTTON_HEIGHT,
            )
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING)
                    .with_flex_child(
                        TextBox::new()
                            .with_text_size(TEXT_SIZE)
                            .with_placeholder("Current OTP...")
                            .lens(ApplicationState::otp_layout.then(OtpLayout::otp))
                            .expand(),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING),
                BUTTON_HEIGHT,
            )
            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
            .with_child(
                Label::new(|data: &ApplicationState, _env: &Env| {
                    data.otp_layout.connect_error.clone()
                })
                .with_text_size(TEXT_SIZE)
                .with_text_alignment(TextAlignment::Center)
                .with_line_break_mode(LineBreaking::WordWrap),
            )
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING)
                    .with_flex_child(
                        Button::from_label(Label::new("Login").with_text_size(TEXT_SIZE))
                            .on_click(OtpLayout::on_connect_clicked)
                            .expand(),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING),
                BUTTON_HEIGHT,
            )
            .with_flex_child(SizedBox::empty().expand(), BOTTOM_PADDING)
    }
    fn on_connect_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // Check if all essential fields are filled.
        if data.otp_layout.otp.is_empty() {
            data.otp_layout.connect_error = String::from("Please, enter your current OTP.");
            return;
        }

        // Try to parse the port string.
        let port = data.connect_layout.port.parse::<u16>();
        if port.is_err() {
            data.connect_layout.port = String::new();
            data.connect_layout.connect_error = String::from("Could not parse port value.");
            data.otp_layout.otp = String::new();
            data.current_layout = Layout::Connect;

            return;
        }
        let port = port.unwrap();

        // Try to connect.
        let result = data.net_service.lock().unwrap().connect(
            data.connect_layout.server.clone(),
            port,
            data.connect_layout.username.clone(),
            data.connect_layout.password.clone(),
            data.otp_layout.otp.clone(),
            None,
        );
        match result {
            ConnectResult::InternalError(app_error) => {
                println!("{}", app_error);
                data.logger_service
                    .lock()
                    .unwrap()
                    .log(&app_error.to_string());
                data.otp_layout.connect_error = app_error.to_string();
            }
            ConnectResult::ConnectFailed(reason) => {
                println!("{}", reason);
                data.logger_service.lock().unwrap().log(&reason);
                data.otp_layout.connect_error = reason;
            }
            ConnectResult::Connected(is_admin) => {
                data.main_layout.is_user_admin = is_admin;
                data.connect_layout.password = String::new();
                data.current_layout = Layout::Main;
            }
            ConnectResult::NeedFirstPassword => {
                let message = "error: received \"NeedFirstPassword\" in OTP mode.";
                println!("{}", message);
                data.logger_service.lock().unwrap().log(&message);
                data.otp_layout.connect_error = String::from(message);
            }
            ConnectResult::SetupOTP(_) => {
                let message = "error: received \"SetupOTP\" in OTP mode.";
                println!("{}", message);
                data.logger_service.lock().unwrap().log(&message);
                data.otp_layout.connect_error = String::from(message);
            }
            ConnectResult::NeedOTP => {
                let message = "error: received \"NeedOTP\" in OTP mode.";
                println!("{}", message);
                data.logger_service.lock().unwrap().log(&message);
                data.otp_layout.connect_error = String::from(message);
            }
        }
    }
}
