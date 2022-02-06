// External.
use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Label, MainAxisAlignment};
use druid::{Lens, TextAlignment, WidgetExt};

// Custom.
use crate::misc::custom_data_button_controller::*;
use crate::ApplicationState;

// Layout customization.
const TITLE_WIDTH: f64 = 0.7;
const DATE_WIDTH: f64 = 0.15;
const TIME_WIDTH: f64 = 0.15;
const TEXT_SIZE: f64 = 18.0;

#[derive(Clone, Data, Lens)]
pub struct ReportWidget {
    id: u64,
    title: String,
    date: String,
    time: String,
    is_hovered: bool,
}

impl ReportWidget {
    pub fn new(id: u64, title: String, date: String, time: String) -> Self {
        Self {
            id,
            title,
            date,
            time,
            is_hovered: false,
        }
    }
    pub fn build_title_ui() -> impl Widget<ApplicationState> {
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_flex_child(
                Label::new("Title")
                    .with_text_alignment(TextAlignment::Start)
                    .with_text_size(TEXT_SIZE)
                    .expand_width(),
                TITLE_WIDTH,
            )
            .with_flex_child(
                Label::new("Date")
                    .with_text_alignment(TextAlignment::Start)
                    .with_text_size(TEXT_SIZE)
                    .expand_width(),
                DATE_WIDTH,
            )
            .with_flex_child(
                Label::new("Time")
                    .with_text_alignment(TextAlignment::Start)
                    .with_text_size(TEXT_SIZE)
                    .expand_width(),
                TIME_WIDTH,
            )
    }
    pub fn build_ui(&self) -> impl Widget<ApplicationState> {
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_flex_child(
                Button::from_label(
                    Label::new(self.title.clone())
                        .with_text_alignment(TextAlignment::Start)
                        .with_text_size(TEXT_SIZE),
                )
                .controller(CustomDataButtonController::new(CustomButtonData {
                    report_id: self.id,
                }))
                .expand_width(),
                TITLE_WIDTH,
            )
            .with_flex_child(
                Label::new(self.date.clone())
                    .with_text_alignment(TextAlignment::Start)
                    .with_text_size(TEXT_SIZE)
                    .expand_width(),
                DATE_WIDTH,
            )
            .with_flex_child(
                Label::new(self.time.clone())
                    .with_text_alignment(TextAlignment::Start)
                    .with_text_size(TEXT_SIZE)
                    .expand_width(),
                TIME_WIDTH,
            )
    }
}
