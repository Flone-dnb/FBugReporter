// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

// Std.
use std::sync::{Arc, Mutex};

// External.
use druid::widget::prelude::*;
use druid::widget::ViewSwitcher;
use druid::{
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Lens, Target, WindowDesc,
};
use rdev::display_size;

// Custom.
use io::log_manager::LogManager;
use layouts::{
    change_password_layout::ChangePasswordLayout, connect_layout::ConnectLayout,
    main_layout::MainLayout, otp_layout::OtpLayout, report_layout::ReportLayout,
    settings_layout::SettingsLayout,
};
use misc::custom_data_button_controller::CUSTOM_DATA_BUTTON_CLICKED;
use misc::theme::*;
use network::net_service::NetService;

mod io;
mod layouts;
mod misc;
mod network;
mod widgets;

#[derive(Clone, Copy, Data, PartialEq)]
pub enum Layout {
    Connect,
    Settings,
    Main,
    ChangePassword,
    Otp,
    Report,
}

#[derive(Clone, Data, Lens)] // Clone is required by `AppDelegate`.
pub struct ApplicationState {
    current_layout: Layout,

    // layouts
    connect_layout: ConnectLayout,
    main_layout: MainLayout,
    settings_layout: SettingsLayout,
    change_password_layout: ChangePasswordLayout,
    otp_layout: OtpLayout,
    report_layout: ReportLayout,

    // services
    #[data(ignore)]
    net_service: Arc<Mutex<NetService>>, // Using `Arc<Mutex>` because need `Clone` and need to modify.
    #[data(ignore)]
    logger_service: Arc<Mutex<LogManager>>, // Using `Arc<Mutex>` because need `Clone` and need to modify.

    // misc
    theme: ApplicationTheme,
}

pub fn main() {
    let window_size = Size {
        width: 750.0,
        height: 500.0,
    };

    let (w, h) = display_size().unwrap();

    // Describe the main window.
    let main_window = WindowDesc::new(build_root_widget())
        .title("FBugReporter - Client")
        .window_size(window_size)
        .set_position((
            w as f64 / 2.0 - window_size.width / 2.0,
            h as f64 / 2.0 - window_size.height / 2.0,
        ));

    // Create the initial app state.
    let initial_state = ApplicationState {
        current_layout: Layout::Connect,
        connect_layout: ConnectLayout::new(),
        main_layout: MainLayout::new(),
        settings_layout: SettingsLayout::new(),
        change_password_layout: ChangePasswordLayout::new(),
        otp_layout: OtpLayout::new(),
        report_layout: ReportLayout::new(),
        net_service: Arc::new(Mutex::new(NetService::new())),
        logger_service: Arc::new(Mutex::new(LogManager::new())),
        theme: ApplicationTheme::new(),
    };

    // Start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .configure_env(apply_theme)
        .delegate(MyDelegate {})
        .launch(initial_state)
        .expect("Failed to launch the application.");
}

fn apply_theme(env: &mut Env, data: &ApplicationState) {
    env.set(
        druid::theme::WINDOW_BACKGROUND_COLOR,
        data.theme.background_color.clone(),
    );
    env.set(
        druid::theme::TEXTBOX_BORDER_RADIUS,
        data.theme.border_radius,
    );
    env.set(druid::theme::BUTTON_BORDER_RADIUS, data.theme.border_radius);
    env.set(
        druid::theme::PLACEHOLDER_COLOR,
        data.theme.placeholder_color.clone(),
    );
    env.set(
        druid::theme::BACKGROUND_LIGHT,
        data.theme.textbox_background_color.clone(),
    );
    env.set(
        druid::theme::BORDER_DARK,
        data.theme.inactive_border_color.clone(),
    );
    env.set(
        druid::theme::SELECTED_TEXT_BACKGROUND_COLOR,
        data.theme.text_selection_color.clone(),
    );
    env.set(
        druid::theme::PRIMARY_LIGHT,
        data.theme.active_border_color.clone(),
    );
    env.set(
        druid::theme::BUTTON_DARK,
        data.theme.button_dark_color.clone(),
    );
    env.set(
        druid::theme::BUTTON_LIGHT,
        data.theme.button_light_color.clone(),
    );
}

fn build_root_widget() -> impl Widget<ApplicationState> {
    ViewSwitcher::new(
        |data: &ApplicationState, _env| data.current_layout,
        |selector, data, _env| match *selector {
            Layout::Connect => Box::new(ConnectLayout::build_ui()),
            Layout::Settings => Box::new(SettingsLayout::build_ui()),
            Layout::Main => Box::new(MainLayout::build_ui()),
            Layout::ChangePassword => Box::new(ChangePasswordLayout::build_ui()),
            Layout::Otp => Box::new(OtpLayout::build_ui(&data.otp_layout)),
            Layout::Report => Box::new(ReportLayout::build_ui(&data)),
        },
    )
}

struct MyDelegate;

impl AppDelegate<ApplicationState> for MyDelegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut ApplicationState,
        _env: &Env,
    ) -> Handled {
        if let Some(button_data) = cmd.get(CUSTOM_DATA_BUTTON_CLICKED) {
            let report = data
                .net_service
                .lock()
                .unwrap()
                .query_report(button_data.report_id);
            if let Err(app_error) = report {
                if app_error.get_message().contains("FIN") {
                    data.current_layout = Layout::Connect;
                    data.connect_layout.connect_error = format!(
                        "{}\nMost likely the server \
                    closed connection due to your inactivity.",
                        app_error.get_message()
                    );
                } else {
                    println!("ERROR: {}", app_error.to_string());
                }

                return Handled::Yes;
            }
            let report = report.unwrap();

            data.report_layout.report = report;
            data.current_layout = Layout::Report;

            Handled::Yes
        } else {
            Handled::No
        }
    }
}
