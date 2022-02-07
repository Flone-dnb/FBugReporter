// External.
use druid::widget::{prelude::*, Button, Padding};
use druid::widget::{CrossAxisAlignment, Flex, Label, LineBreaking, SizedBox};

// Custom.
use crate::{ApplicationState, Layout};

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
                    Label::new(
                        String::from("FBugReporter - Client (v") + env!("CARGO_PKG_VERSION") + ")",
                    )
                    .with_line_break_mode(LineBreaking::WordWrap)
                    .with_text_size(TEXT_SIZE),
                    1.0,
                )
                .with_default_spacer()
                .with_flex_child(
                    Flex::row()
                        .with_child(
                            Label::new("The source code is available ").with_text_size(TEXT_SIZE),
                        )
                        .with_child(
                            Button::from_label(
                                Label::new("here")
                                    .with_line_break_mode(LineBreaking::WordWrap)
                                    .with_text_size(TEXT_SIZE),
                            )
                            .on_click(|_ctx, _data, _env| {
                                opener::open("https://github.com/Flone-dnb/FBugReporter").unwrap()
                            }),
                        ),
                    1.0,
                )
                .with_default_spacer()
                .with_default_spacer()
                .with_flex_child(
                    Label::new(|data: &ApplicationState, _env: &Env| "Log file location: TODO.")
                        .with_text_size(TEXT_SIZE),
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
                .with_flex_child(SizedBox::empty().expand(), 10.0)
                .with_flex_child(
                    Button::from_label(
                        Label::new("back")
                            .with_line_break_mode(LineBreaking::WordWrap)
                            .with_text_size(TEXT_SIZE),
                    )
                    .on_click(|_ctx, data: &mut ApplicationState, _env| {
                        data.current_layout = Layout::Connect;
                    }),
                    1.0,
                ),
        )
    }
}
