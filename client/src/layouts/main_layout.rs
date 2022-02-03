// External.
use druid::widget::prelude::*;
use druid::widget::{Flex, Label, MainAxisAlignment};

// Custom.
use crate::widgets::report::ReportWidget;
use crate::ApplicationState;

#[derive(Clone, Data)]
pub struct MainLayout {}

impl MainLayout {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        let report = ReportWidget::new(
            String::from("test report"),
            String::from("03.02.2022"),
            String::from("23:21"),
        );

        Flex::column()
            .main_axis_alignment(MainAxisAlignment::Center)
            .must_fill_main_axis(true)
            .with_flex_child(
                Flex::column()
                    .with_flex_child(report.build_ui(), 1.0)
                    .with_flex_child(report.build_ui(), 1.0)
                    .with_flex_child(report.build_ui(), 1.0)
                    .with_flex_child(report.build_ui(), 1.0),
                1.0,
            )
    }
}
