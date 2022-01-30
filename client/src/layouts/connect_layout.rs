// External.
use druid::widget::prelude::*;
use druid::widget::{Flex, Label, MainAxisAlignment};

// Custom.
use crate::ApplicationState;

#[derive(Clone, Data)]
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
            .with_flex_child(Label::new("Hello World from connect layout!"), 100.0)
    }
}
