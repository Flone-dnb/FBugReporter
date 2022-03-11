// External.
use druid::widget::{prelude::*, SizedBox};
use druid::widget::{Button, Flex, Label, LineBreaking, MainAxisAlignment, TextBox};
use druid::{Lens, LensExt, TextAlignment, WidgetExt};

// Custom.
use crate::services::{
    net_packets::*,
    net_service::{ConnectResult, NETWORK_PROTOCOL_VERSION},
};
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
                data.change_password_layout.connect_error = _message;
            }
            ConnectResult::Connected => {
                data.current_layout = Layout::Main;
            }
        }
    }
}
