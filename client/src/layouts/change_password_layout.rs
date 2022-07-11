// External.
use druid::widget::{prelude::*, SizedBox};
use druid::widget::{Button, Flex, Label, LineBreaking, MainAxisAlignment, TextBox};
use druid::{Lens, LensExt, TextAlignment, WidgetExt};

// Custom.
use crate::network::net_service::ConnectResult;
use crate::{ApplicationState, Layout};

// Layout customization.
const WIDTH_PADDING: f64 = 0.25;
const LEFT_SIDE_SIZE: f64 = 0.5;
const RIGHT_SIDE_SIZE: f64 = 0.8;
const TOP_PADDING: f64 = 0.5;
const BOTTOM_PADDING: f64 = 0.75;
const ROW_SPACING: f64 = 0.3;
const BUTTONS_WIDTH_PADDING: f64 = 1.0;
const BUTTON_HEIGHT: f64 = 0.3;
const TEXT_SIZE: f64 = 20.0;

#[derive(Clone, Data, Lens)]
pub struct ChangePasswordLayout {
    pub old_password: String,
    pub new_password: String,
    pub new_password_repeat: String,
    connect_error: String,
}

impl ChangePasswordLayout {
    pub fn new() -> Self {
        Self {
            old_password: String::new(),
            new_password: String::new(),
            new_password_repeat: String::new(),
            connect_error: String::new(),
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(SizedBox::empty().expand(), TOP_PADDING)
            .with_child(Label::new("Set New Password").with_text_size(TEXT_SIZE))
            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
            .with_flex_child(
                Flex::row()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .must_fill_main_axis(true)
                    .with_flex_child(SizedBox::empty().expand(), WIDTH_PADDING)
                    .with_flex_child(
                        Flex::column()
                            .with_flex_child(
                                Label::new("Old Password:")
                                    .with_text_size(TEXT_SIZE)
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                Label::new("New Password:")
                                    .with_text_size(TEXT_SIZE)
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                Label::new("Repeat New Password:")
                                    .with_text_size(TEXT_SIZE)
                                    .expand(),
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
                                    .with_placeholder("Your old password...")
                                    .lens(
                                        ApplicationState::change_password_layout
                                            .then(ChangePasswordLayout::old_password),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("Your new password...")
                                    .lens(
                                        ApplicationState::change_password_layout
                                            .then(ChangePasswordLayout::new_password),
                                    )
                                    .expand(),
                                1.0,
                            )
                            .with_flex_child(SizedBox::empty().expand(), ROW_SPACING)
                            .with_flex_child(
                                TextBox::new()
                                    .with_text_size(TEXT_SIZE)
                                    .with_placeholder("Repeat your new password...")
                                    .lens(
                                        ApplicationState::change_password_layout
                                            .then(ChangePasswordLayout::new_password_repeat),
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
                    data.change_password_layout.connect_error.clone()
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
                        Button::from_label(Label::new("Change Password").with_text_size(TEXT_SIZE))
                            .on_click(ChangePasswordLayout::on_change_password_clicked)
                            .expand(),
                        1.0,
                    )
                    .with_flex_child(SizedBox::empty().expand(), BUTTONS_WIDTH_PADDING),
                BUTTON_HEIGHT,
            )
            .with_flex_child(SizedBox::empty().expand(), BOTTOM_PADDING)
    }
    fn on_change_password_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        if data.change_password_layout.new_password
            != data.change_password_layout.new_password_repeat
        {
            data.change_password_layout.connect_error = String::from("New passwords do not match.");
            return;
        }

        // Try to connect.
        let result = data.net_service.lock().unwrap().connect(
            data.connect_layout.server.clone(),
            data.connect_layout.port.parse::<u16>().unwrap(),
            data.connect_layout.username.clone(),
            data.change_password_layout.old_password.clone(),
            String::new(),
            Some(data.change_password_layout.new_password.clone()),
        );

        match result {
            ConnectResult::InternalError(app_error) => {
                println!("{}", app_error);
                data.logger_service
                    .lock()
                    .unwrap()
                    .log(&app_error.to_string());
                data.change_password_layout.connect_error = app_error.to_string();
            }
            ConnectResult::ConnectFailed(reason) => {
                println!("{}", reason);
                data.logger_service.lock().unwrap().log(&reason);
                data.change_password_layout.connect_error = reason;
            }
            ConnectResult::NeedFirstPassword => {
                let message = "error: received \"NeedFirstPassword\" in Change Password mode.";
                println!("{}", message);
                data.logger_service.lock().unwrap().log(&message);
                data.change_password_layout.connect_error = String::from(message);
            }
            ConnectResult::SetupOTP(qr_code) => {
                data.otp_layout.qr_code = Some(qr_code);

                // set password for OTP layout.
                data.connect_layout.password = data.change_password_layout.new_password.clone();

                data.change_password_layout.new_password_repeat = String::new();
                data.change_password_layout.new_password = String::new();
                data.change_password_layout.old_password = String::new();

                data.current_layout = Layout::Otp;
            }
            ConnectResult::NeedOTP => {
                // set password for OTP layout.
                data.connect_layout.password = data.change_password_layout.new_password.clone();

                data.change_password_layout.new_password_repeat = String::new();
                data.change_password_layout.new_password = String::new();
                data.change_password_layout.old_password = String::new();

                data.current_layout = Layout::Otp;
            }
            ConnectResult::Connected(is_admin) => {
                data.main_layout.is_user_admin = is_admin;

                data.change_password_layout.new_password_repeat = String::new();
                data.change_password_layout.new_password = String::new();
                data.change_password_layout.old_password = String::new();

                data.connect_layout.password = String::new();

                data.current_layout = Layout::Main;
            }
        }
    }
}
