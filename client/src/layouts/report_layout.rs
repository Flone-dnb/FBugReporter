// Std.
use std::fs::File;
use std::io::Write;

// External.
use druid::widget::{prelude::*, Scroll, SizedBox};
use druid::widget::{Button, Flex, Label, Padding};
use druid::WidgetExt;
use native_dialog::{FileDialog, MessageDialog, MessageType};

// Custom.
use super::main_layout::REPORT_COUNT_PER_PAGE;
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
                        .with_flex_child(delete_report_section, 1.0)
                        .with_child(
                            Button::from_label(
                                Label::new("Save to File").with_text_size(TEXT_SIZE),
                            )
                            .on_click(ReportLayout::on_save_to_file_clicked),
                        ),
                    0.2,
                ),
        )
    }
    fn on_return_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        // Hack: do this here because query_reports from MainLayout
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
                    "{}\nMost likely the server \
                    closed connection due to your inactivity.",
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
        if result.is_ok() {
            let (reports, total_count) = result.unwrap();
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
                    "{}\nMost likely the server \
                    closed connection due to your inactivity.",
                    app_error.get_message()
                );
            } else {
                println!(
                    "ERROR: {}",
                    app_error.add_entry(file!(), line!()).to_string()
                );
            }

            return;
        }
        let found = result.unwrap();

        if found == false {
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
            println!("FileDialog returned None");
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
