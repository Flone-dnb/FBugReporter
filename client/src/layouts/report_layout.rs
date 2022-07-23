// Std.
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

// External.
use druid::widget::{prelude::*, Scroll, SizedBox};
use druid::widget::{Button, Flex, Label, Padding};
use druid::{TextAlignment, WidgetExt};
use native_dialog::{FileDialog, MessageDialog, MessageType};

// Custom.
use super::main_layout::REPORT_COUNT_PER_PAGE;
use crate::misc::report_attachment_button::*;
use crate::{ApplicationState, Layout};
use shared::misc::report::ReportData;

// Layout customization.
const TEXT_SIZE: f64 = 18.0;

#[derive(Clone)]
pub struct ReportLayout {
    pub report: Rc<ReportData>, // using Rc to implement Clone
}

impl ReportLayout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_ui(data: &ApplicationState) -> impl Widget<ApplicationState> {
        let mut delete_report_section: Flex<ApplicationState> = Flex::row();
        if data.main_layout.is_user_admin {
            delete_report_section = delete_report_section
                .with_flex_child(SizedBox::empty().expand_width(), 0.5)
                .with_child(
                    Button::from_label(Label::new("Delete Report").with_text_size(TEXT_SIZE))
                        .on_click(ReportLayout::on_delete_clicked),
                )
                .with_flex_child(SizedBox::empty().expand_width(), 0.5);
        } else {
            delete_report_section =
                delete_report_section.with_flex_child(SizedBox::empty().expand_width(), 1.0)
        }

        // Setup attachment column.
        let mut attachment_column = Flex::column();

        if !data.report_layout.report.attachments.is_empty() {
            attachment_column.add_child(
                Label::new("Report attachments:")
                    .with_text_size(TEXT_SIZE)
                    .align_left(),
            );
            attachment_column.add_default_spacer();
            for attachment in data.report_layout.report.attachments.iter() {
                attachment_column.add_child(
                    Flex::row()
                        .with_child(
                            Button::from_label(
                                Label::new(attachment.file_name.clone())
                                    .with_text_alignment(TextAlignment::Start)
                                    .with_text_size(TEXT_SIZE),
                            )
                            .controller(
                                ReportAttachmentButtonController::new(ReportAttachmentButtonData {
                                    attachment_id: attachment.id,
                                    attachment_file_name: attachment.file_name.clone(),
                                }),
                            ),
                        )
                        .with_child(
                            Label::new(match attachment.size_in_bytes {
                                0..=1023 => format!("{} bytes", attachment.size_in_bytes),
                                1024..=1048575 => format!("{} KB", attachment.size_in_bytes / 1024),
                                _ => format!("{} MB", attachment.size_in_bytes / 1024 / 1024),
                            })
                            .with_text_size(TEXT_SIZE),
                        )
                        .align_left(),
                );
            }
        }

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
                                    String::from("Sender: no information provided")
                                } else if data.report_layout.report.sender_email.is_empty() {
                                    format!("Sender: {}", data.report_layout.report.sender_name)
                                } else {
                                    format!(
                                        "Sender: {} ({})",
                                        data.report_layout.report.sender_name,
                                        data.report_layout.report.sender_email
                                    )
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
                .with_child(attachment_column)
                .with_default_spacer()
                .with_default_spacer()
                .with_flex_child(
                    Flex::row()
                        .with_child(
                            Button::from_label(Label::new("Return").with_text_size(TEXT_SIZE))
                                .on_click(ReportLayout::on_return_clicked),
                        )
                        .with_flex_child(delete_report_section, 1.0)
                        .with_child(
                            Button::from_label(
                                Label::new("Save Text to File").with_text_size(TEXT_SIZE),
                            )
                            .on_click(ReportLayout::on_save_to_file_clicked),
                        ),
                    0.2,
                ),
        )
    }
    fn on_return_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // Do this here because query_reports from MainLayout
        // does not have mut Data.
        let result = data
            .net_service
            .lock()
            .unwrap()
            .query_reports(data.main_layout.current_page, REPORT_COUNT_PER_PAGE);

        if let Err(app_error) = result {
            if app_error.get_message().contains("FIN") {
                data.current_layout = Layout::Connect;
                data.connect_layout.connect_error = format!(
                    "{}\nMaybe the server \
                    closed the connection due to your inactivity.",
                    app_error.get_message()
                );
            } else {
                data.logger_service
                    .lock()
                    .unwrap()
                    .log(&app_error.to_string());
            }

            return;
        }
        if let Ok((reports, total_count)) = result {
            *data.main_layout.reports.borrow_mut() = reports;
            data.main_layout.total_reports.set(total_count);
        }

        //data.main_layout.reports.borrow_mut().clear(); // will refresh reports list
        data.current_layout = Layout::Main;
    }
    fn on_delete_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        let yes = MessageDialog::new()
            .set_type(MessageType::Info)
            .set_title(&format!("Report #{}", data.report_layout.report.id))
            .set_text("Are you sure you want to delete this report?")
            .show_confirm()
            .unwrap();
        if !yes {
            return;
        }

        let result = data
            .net_service
            .lock()
            .unwrap()
            .delete_report(data.report_layout.report.id);
        if let Err(app_error) = result {
            if app_error.get_message().contains("FIN") {
                data.current_layout = Layout::Connect;
                data.connect_layout.connect_error = format!(
                    "{}\nMaybe the server \
                    closed the connection due to your inactivity.",
                    app_error.get_message()
                );
            } else {
                println!("ERROR: {}", app_error);
            }

            return;
        }
        let found = result.unwrap();

        if !found {
            println!(
                "ERROR: a report with id {} was not found",
                data.report_layout.report.id
            );
        } else {
            data.main_layout.reports.borrow_mut().clear(); // will refresh reports list
            data.current_layout = Layout::Main;
        }
    }
    fn on_save_to_file_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        let path = FileDialog::new()
            .add_filter("Text file", &["txt"])
            .set_filename(&format!("Report #{}.txt", data.report_layout.report.id))
            .show_save_single_file()
            .unwrap();
        if path.is_none() {
            return;
        }
        let path = path.unwrap();

        println!(
            "Received path of the file to save report:\npath:{}\nreport id:{}",
            path.to_str().unwrap(),
            data.report_layout.report.id
        );

        let mut file = File::create(path.to_str().unwrap()).unwrap();
        writeln!(&mut file, "id: {}", data.report_layout.report.id).unwrap();
        writeln!(&mut file, "title: {}", data.report_layout.report.title).unwrap();
        writeln!(
            &mut file,
            "game_name: {}",
            data.report_layout.report.game_name
        )
        .unwrap();
        writeln!(
            &mut file,
            "game_version: {}",
            data.report_layout.report.game_version
        )
        .unwrap();
        writeln!(&mut file, "date: {}", data.report_layout.report.date).unwrap();
        writeln!(&mut file, "time: {}", data.report_layout.report.time).unwrap();
        writeln!(
            &mut file,
            "sender_name: {}",
            data.report_layout.report.sender_name
        )
        .unwrap();
        writeln!(
            &mut file,
            "sender_email: {}",
            data.report_layout.report.sender_email
        )
        .unwrap();
        writeln!(&mut file, "os_info: {}", data.report_layout.report.os_info).unwrap();
        writeln!(&mut file, "text:\n{}", data.report_layout.report.text).unwrap();
    }
}

impl Default for ReportLayout {
    fn default() -> Self {
        Self {
            report: Rc::new(ReportData {
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
                attachments: Vec::new(),
            }),
        }
    }
}
