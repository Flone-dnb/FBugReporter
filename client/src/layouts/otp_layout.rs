// Std.
use std::time::SystemTime;

// External.
use druid::widget::{prelude::*, SizedBox};
use druid::widget::{Button, Flex, Label, LineBreaking, MainAxisAlignment, TextBox};
use druid::{
    piet::{ImageBuf, ImageFormat, InterpolationMode},
    widget::{FillStrat, Image},
};
use druid::{Lens, LensExt, TextAlignment, WidgetExt};
use totp_rs::{Algorithm, TOTP};

// Custom.
use crate::services::{
    net_packets::*,
    net_service::{ConnectResult, NETWORK_PROTOCOL_VERSION},
};
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

fn get_otp_with_totp() -> String {
    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, "supersecret");
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let token = totp.generate(time);
    totp.get_qr("FBugReporter", "username").unwrap()
}

#[derive(Clone, Data, Lens)]
pub struct OtpLayout {
    otp: String,
    connect_error: String,
}

impl OtpLayout {
    pub fn new() -> Self {
        Self {
            otp: String::new(),
            connect_error: String::new(),
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        let image = photon_rs::base64_to_image(&get_otp_with_totp());
        let pixels = image.get_raw_pixels();
        let image_data = ImageBuf::from_raw(
            pixels,
            ImageFormat::RgbaSeparate,
            image.get_width() as usize,
            image.get_width() as usize,
        );

        let image_widget = Image::new(image_data)
            // set the fill strategy
            .fill_mode(FillStrat::Fill)
            // set the interpolation mode
            .interpolation_mode(InterpolationMode::Bilinear);

        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(SizedBox::empty().expand(), TOP_PADDING)
            .with_flex_child(image_widget.expand(), 1.0)
            .with_default_spacer()
            .with_flex_child(
                Label::new("Enter your OTP here:")
                    .with_text_size(TEXT_SIZE)
                    .expand(),
                1.0,
            )
            .with_flex_child(
                TextBox::new()
                    .with_text_size(TEXT_SIZE)
                    .with_placeholder("Current OTP...")
                    .lens(ApplicationState::otp_layout.then(OtpLayout::otp))
                    .expand(),
                1.0,
            )
            .with_flex_child(
                Button::from_label(Label::new("Connect").with_text_size(TEXT_SIZE))
                    .on_click(OtpLayout::on_connect_clicked)
                    .expand(),
                1.0,
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
                let mut _message = String::new();

                match reason {
                    ClientLoginFailReason::WrongProtocol { server_protocol } => {
                        _message = format!(
                            "Failed to connect to the server \
                            due to incompatible application version.\n\
                            Your application uses network protocol version {}, \
                            while the server supports version {}.",
                            NETWORK_PROTOCOL_VERSION, server_protocol
                        );
                    }
                    ClientLoginFailReason::WrongCredentials { result } => match result {
                        ClientLoginFailResult::FailedAttempt {
                            failed_attempts_made,
                            max_failed_attempts,
                        } => {
                            _message = format!(
                                "Incorrect login/password.\n\
                                Allowed failed login attempts: {0} out of {1}.\n\
                                After {1} failed login attempts new failed login attempt \
                                 will result in a ban.",
                                failed_attempts_made, max_failed_attempts
                            );
                        }
                        ClientLoginFailResult::Banned { ban_time_in_min } => {
                            _message = format!(
                                "You were banned due to multiple failed login attempts.\n\
                                Ban time: {} minute(-s).\n\
                                During this time the server will reject any \
                                login attempts without explanation.",
                                ban_time_in_min
                            );
                        }
                    },
                    ClientLoginFailReason::NeedFirstPassword => {
                        _message = String::from("Need to set the first password.");
                        data.current_layout = Layout::ChangePassword;
                    }
                }

                println!("{}", _message);
                data.logger_service.lock().unwrap().log(&_message);
                data.connect_layout.connect_error = _message;
            }
            ConnectResult::Connected => {
                data.connect_layout.password = String::new();

                data.current_layout = Layout::Main;
            }
        }
    }
}
