// External.
use druid::widget::prelude::*;
use druid::widget::{Align, Button, Flex, Label, MainAxisAlignment, SizedBox};
use druid::{Lens, UnitPoint, WidgetExt};

// Custom.
use crate::ApplicationState;

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
    pub fn build_ui(&self) -> impl Widget<ApplicationState> {
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_child(Label::new(self.title.clone()).with_text_size(TEXT_SIZE))
            .with_flex_child(SizedBox::empty().expand(), 1.0)
            .with_child(Label::new(self.date.clone()).with_text_size(TEXT_SIZE))
            .with_child(Label::new(self.time.clone()).with_text_size(TEXT_SIZE))
            .on_click(|_ctx, _data: &mut ApplicationState, _env| println!("clicked"))
    }
}
