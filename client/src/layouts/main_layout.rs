// External.
use druid::widget::prelude::*;
use druid::widget::{Flex, Label, MainAxisAlignment, Padding};

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
            String::from("12345678901234567890123456789012345678901234567890"),
            String::from("03.02.2022"),
            String::from("23:21"),
        );

        Padding::new(
            5.0,
            Flex::column()
                .main_axis_alignment(MainAxisAlignment::Start)
                .must_fill_main_axis(true)
                .with_flex_child(
                    Flex::column()
                        .with_child(ReportWidget::build_title_ui())
                        .with_default_spacer()
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui()),
                    1.0,
                ),
        )
    }
}
