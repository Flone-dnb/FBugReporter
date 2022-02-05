// External.
use druid::widget::prelude::*;
use druid::widget::{Button, Container, Flex, Label, MainAxisAlignment};
use druid::{Lens, TextAlignment, WidgetExt};

// Custom.
use crate::ApplicationState;

// Layout customization.
const TITLE_WIDTH: f64 = 0.7;
const DATE_WIDTH: f64 = 0.15;
const TIME_WIDTH: f64 = 0.15;
const TEXT_SIZE: f64 = 18.0;

#[derive(Clone, Data, Lens)]
pub struct ReportWidget {
    title: String,
    date: String,
    time: String,
}

impl ReportWidget {
    pub fn new(title: String, date: String, time: String) -> Self {
        Self { title, date, time }
    }
    pub fn build_title_ui() -> impl Widget<ApplicationState> {
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_flex_child(
                Container::new(
                    Label::new("Title")
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE),
                )
                .border(druid::theme::PRIMARY_LIGHT, 1.0)
                .expand_width(),
                TITLE_WIDTH,
            )
            .with_flex_child(
                Container::new(
                    Label::new("Date")
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE),
                )
                .border(druid::theme::PRIMARY_LIGHT, 1.0)
                .expand_width(),
                DATE_WIDTH,
            )
            .with_flex_child(
                Container::new(
                    Label::new("Time")
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE),
                )
                .border(druid::theme::PRIMARY_LIGHT, 1.0)
                .expand_width(),
                TIME_WIDTH,
            )
    }
    pub fn build_ui(&self) -> impl Widget<ApplicationState> {
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_flex_child(
                Container::new(
                    Label::new(self.title.clone())
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE)
                        .on_click(|_ctx, _data: &mut ApplicationState, _env| println!("clicked")),
                )
                .border(druid::theme::PRIMARY_LIGHT, 1.0)
                .expand_width(),
                TITLE_WIDTH,
            )
            .with_flex_child(
                Container::new(
                    Label::new(self.date.clone())
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE)
                        .expand_width(),
                )
                .border(druid::theme::PRIMARY_LIGHT, 1.0)
                .expand_width(),
                DATE_WIDTH,
            )
            .with_flex_child(
                Container::new(
                    Label::new(self.time.clone())
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE)
                        .expand_width(),
                )
                .border(druid::theme::PRIMARY_LIGHT, 1.0)
                .expand_width(),
                TIME_WIDTH,
            )
    }
}
