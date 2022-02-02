use druid::WidgetExt;
// External.
use druid::widget::{prelude::*, Button, Padding};
use druid::widget::{CrossAxisAlignment, Flex, Label, LineBreaking, MainAxisAlignment};

// Custom.
use crate::ApplicationState;

const TEXT_SIZE: f64 = 20.0;

#[derive(Clone, Data)]
pub struct SettingsLayout {}

impl SettingsLayout {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        Padding::new(
            10.0,
            Flex::column()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .must_fill_main_axis(true)
                .with_flex_child(
                    Label::new("Log file location: TODO").with_text_size(TEXT_SIZE),
                    1.0,
                )
                .with_default_spacer()
                .with_flex_child(
                    Label::new(
                        "Theme can be customized by copy-pasting \
                    the 'theme.ini' file from the repository's 'client' folder \
                    next to the client's executable file.",
                    )
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(TEXT_SIZE),
                    1.0,
                )
                .with_default_spacer()
                .with_flex_child(
                    Label::new("TODO: author/github repo button")
                        .with_line_break_mode(LineBreaking::WordWrap)
                        .with_text_size(TEXT_SIZE),
                    1.0,
                )
                .with_flex_child(
                    Button::from_label(
                        Label::new("back")
                            .with_line_break_mode(LineBreaking::WordWrap)
                            .with_text_size(TEXT_SIZE),
                    ),
                    1.0,
                ),
        )
    }
}
