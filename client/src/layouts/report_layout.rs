// External.
use druid::widget::{prelude::*, Scroll, SizedBox};
use druid::widget::{Button, Flex, Label, Padding};
use druid::WidgetExt;

// Custom.
use crate::{ApplicationState, Layout};

#[derive(Clone, Data)]
pub struct ReportData {
    pub id: u64,
    pub title: String,
    pub game_name: String,
    pub game_version: String,
    pub text: String,
    pub date: String,
    pub time: String,
    pub sender_name: String,
    pub sender_email: String,
    pub os_info: String,
}

// Layout customization.
const TEXT_SIZE: f64 = 18.0;

#[derive(Clone, Data)]
pub struct ReportLayout {
    pub report: ReportData,
}

impl ReportLayout {
    pub fn new() -> Self {
        Self {
            report: ReportData {
                id: 0,
                title: String::new(),
                game_name: String::new(),
                game_version: String::new(),
                text: String::new(),
                date: String::new(),
                time: String::new(),
                sender_name: String::new(),
                sender_email: String::new(),
                os_info: String::new(),
            },
        }
    }

    pub fn build_ui() -> impl Widget<ApplicationState> {
        Padding::new(
            5.0,
            Flex::column()
                .must_fill_main_axis(true)
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &_| {
                                format!(
                                    "Title: {} (id: {})",
                                    data.report_layout.report.title, data.report_layout.report.id
                                )
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .align_left(),
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &_| {
                                format!(
                                    "Game: {} (version: {})",
                                    data.report_layout.report.game_name,
                                    data.report_layout.report.game_version
                                )
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .align_left(),
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &_| {
                                format!(
                                    "Date and time: {}, {}",
                                    data.report_layout.report.date, data.report_layout.report.time
                                )
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .align_left(),
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &_| {
                                if data.report_layout.report.sender_name.is_empty()
                                    && data.report_layout.report.sender_email.is_empty()
                                {
                                    format!("Sender: no information provided")
                                } else {
                                    if !data.report_layout.report.sender_name.is_empty()
                                        && data.report_layout.report.sender_email.is_empty()
                                    {
                                        format!("Sender: {}", data.report_layout.report.sender_name)
                                    } else if data.report_layout.report.sender_name.is_empty()
                                        && !data.report_layout.report.sender_email.is_empty()
                                    {
                                        format!(
                                            "Sender: email: {}",
                                            data.report_layout.report.sender_name
                                        )
                                    } else {
                                        format!(
                                            "Sender: {} (email: {})",
                                            data.report_layout.report.sender_name,
                                            data.report_layout.report.sender_email
                                        )
                                    }
                                }
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .align_left(),
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &ApplicationState, _env: &_| {
                                format!("OS info: {}", data.report_layout.report.os_info)
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .align_left(),
                )
                .with_default_spacer()
                .with_default_spacer()
                .with_flex_child(
                    Scroll::new(
                        Label::new(|data: &ApplicationState, _env: &_| {
                            data.report_layout.report.text.clone()
                        })
                        .with_text_size(TEXT_SIZE)
                        .align_left(),
                    )
                    .vertical(),
                    1.0,
                )
                .with_default_spacer()
                .with_default_spacer()
                .with_flex_child(
                    Flex::row()
                        .with_child(
                            Button::from_label(Label::new("Return").with_text_size(TEXT_SIZE))
                                .on_click(ReportLayout::on_return_clicked),
                        )
                        .with_flex_child(SizedBox::empty().expand_width(), 1.0)
                        .with_child(
                            Button::from_label(
                                Label::new("Save to file").with_text_size(TEXT_SIZE),
                            )
                            .on_click(ReportLayout::on_save_to_file_clicked),
                        ),
                    0.2,
                ),
        )
    }
    fn on_return_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        data.main_layout.reports.borrow_mut().clear(); // will refresh reports list
        data.current_layout = Layout::Main;
    }
    fn on_save_to_file_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // TODO
    }
}
