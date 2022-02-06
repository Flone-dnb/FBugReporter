// External.
use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Label, MainAxisAlignment, Padding};
use druid::WidgetExt;

// Custom.
use crate::widgets::report::ReportWidget;
use crate::{ApplicationState, Layout};

// Layout customization.
const TEXT_SIZE: f64 = 16.0;

#[derive(Clone, Data)]
pub struct MainLayout {
    pub refresh_ui: bool,
}

impl MainLayout {
    pub fn new() -> Self {
        Self { refresh_ui: false }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        let report = ReportWidget::new(
            0,
            String::from("12345678901234567890123456789012345678901234567890"),
            String::from("03.02.2022"),
            String::from("23:21"),
        );

        Padding::new(
            5.0,
            Flex::column()
                .main_axis_alignment(MainAxisAlignment::Start)
                .must_fill_main_axis(true)
                .with_child(
                    Button::from_label(Label::new("settings").with_text_size(TEXT_SIZE))
                        .on_click(|_ctx, data: &mut ApplicationState, _env| {
                            data.current_layout = Layout::Settings
                        })
                        .align_left(),
                )
                .with_default_spacer()
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
