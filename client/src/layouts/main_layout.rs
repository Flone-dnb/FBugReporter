// External.
use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Label, MainAxisAlignment, Padding};
use druid::WidgetExt;

// Custom.
use crate::widgets::report::ReportWidget;
use crate::ApplicationState;

// Layout customization.
const TEXT_SIZE: f64 = 18.0;

#[derive(Clone, Data)]
pub struct MainLayout {
    pub current_page: u64,
}

impl MainLayout {
    pub fn new() -> Self {
        Self { current_page: 1 }
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
                .with_flex_child(
                    Flex::column()
                        .with_child(ReportWidget::build_title_ui())
                        .with_default_spacer()
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui())
                        .with_child(report.build_ui()),
                    1.0,
                )
                .with_default_spacer()
                .with_child(
                    Flex::row()
                        .must_fill_main_axis(true)
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .with_flex_child(
                            Button::from_label(
                                Label::new("Show First Page").with_text_size(TEXT_SIZE),
                            )
                            .disabled_if(|data: &ApplicationState, _env| {
                                data.main_layout.current_page == 1
                            })
                            .align_left(),
                            0.25,
                        )
                        .with_flex_child(
                            Flex::column()
                                .with_child(Label::new("page").with_text_size(TEXT_SIZE))
                                .with_child(
                                    Flex::row()
                                        .with_child(
                                            Button::from_label(
                                                Label::new("<").with_text_size(TEXT_SIZE),
                                            )
                                            .disabled_if(|data: &ApplicationState, _env| {
                                                data.main_layout.current_page == 1
                                            }),
                                        )
                                        .with_child(Label::new("P").with_text_size(TEXT_SIZE))
                                        .with_child(Button::from_label(
                                            Label::new(">").with_text_size(TEXT_SIZE),
                                        )),
                                )
                                .with_child(Label::new("out of N").with_text_size(TEXT_SIZE)),
                            0.5,
                        )
                        .with_flex_child(
                            Button::from_label(
                                Label::new("Show Last Page").with_text_size(TEXT_SIZE),
                            )
                            .align_right(),
                            0.25,
                        ),
                ),
        )
    }
}
