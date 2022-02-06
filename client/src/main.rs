// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

// External.
use druid::widget::prelude::*;
use druid::widget::ViewSwitcher;
use druid::{
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Lens, Target, WindowDesc,
};
use rdev::display_size;

// Custom.
use layouts::connect_layout::ConnectLayout;
use layouts::main_layout::MainLayout;
use layouts::settings_layout::SettingsLayout;
use misc::custom_data_button_controller::CUSTOM_DATA_BUTTON_CLICKED;
use theme::*;

mod layouts;
mod misc;
mod theme;
mod widgets;

#[derive(Clone, Copy, Data, PartialEq)]
pub enum Layout {
    Connect,
    Settings,
    Main,
}
#[derive(Clone, Data, Lens)]
pub struct ApplicationState {
    current_layout: Layout,
    connect_layout: ConnectLayout,
    main_layout: MainLayout,
    settings_layout: SettingsLayout,
    theme: ApplicationTheme,
    is_connected: bool,
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
        theme: ApplicationTheme::new(),
        is_connected: false,
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
        |selector, _data, _env| match *selector {
            Layout::Connect => Box::new(ConnectLayout::build_ui()),
            Layout::Settings => Box::new(SettingsLayout::build_ui()),
            Layout::Main => Box::new(MainLayout::build_ui()),
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
            println!("open report with id {}", button_data.report_id);
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
