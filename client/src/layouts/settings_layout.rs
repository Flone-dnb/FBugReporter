// External.
use druid::widget::prelude::*;
use druid::widget::{Flex, Label, MainAxisAlignment};

// Custom.
use crate::ApplicationState;

#[derive(Clone, Data)]
pub struct SettingsLayout {}

impl SettingsLayout {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(Label::new("Hello World from settings layout!"), 100.0)
    }
}
