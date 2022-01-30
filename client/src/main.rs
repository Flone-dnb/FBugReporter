// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

// External.
use druid::widget::prelude::*;
use druid::widget::ViewSwitcher;
use druid::{AppLauncher, Data, WindowDesc};
use rdev::display_size;

// Custom.
use layouts::connect_layout::ConnectLayout;
use layouts::main_layout::MainLayout;
use layouts::settings_layout::SettingsLayout;

mod layouts;

#[derive(Clone, Copy, Data, PartialEq)]
pub enum Layout {
    Connect,
    Settings,
    Main,
}
#[derive(Clone, Data)]
pub struct ApplicationState {
    current_layout: Layout,
}

pub fn main() {
    let window_size = Size {
        width: 650.0,
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
    };

    // Start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch the application.");
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
