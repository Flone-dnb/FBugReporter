// Std.
use std::cell::RefCell;
use std::rc::Rc;

// External.
use druid::widget::{prelude::*, ViewSwitcher};
use druid::widget::{Button, Flex, Label, MainAxisAlignment, Padding};
use druid::WidgetExt;

// Custom.
use crate::services::net_packets::ReportSummary;
use crate::widgets::report::ReportWidget;
use crate::ApplicationState;

// Layout customization.
const TEXT_SIZE: f64 = 18.0;
const REPORT_COUNT_PER_PAGE: u64 = 15;

#[derive(Clone, Data)]
pub struct MainLayout {
    pub current_page: u64,

    #[data(ignore)]
    reports: Rc<RefCell<Vec<ReportSummary>>>, // using Rc because Data requires Clone
}

impl MainLayout {
    pub fn new() -> Self {
        Self {
            current_page: 1,
            reports: Rc::new(RefCell::new(Vec::new())),
        }
    }
    pub fn build_ui() -> impl Widget<ApplicationState> {
        ViewSwitcher::new(
            // repaint ui when current page changed
            |data: &ApplicationState, _env| data.main_layout.current_page,
            |selector, data, _env| match selector {
                _ => Box::new(MainLayout::build_ui_internal(data)),
            },
        )
    }
    fn build_ui_internal(data: &ApplicationState) -> impl Widget<ApplicationState> {
        if data.main_layout.reports.borrow_mut().is_empty() {
            let result = data.main_layout.query_reports(data);
            if result.is_ok() {
                *data.main_layout.reports.borrow_mut() = result.unwrap();
            }
        }

        let mut reports_column = Flex::column()
            .with_child(ReportWidget::build_title_ui())
            .with_default_spacer();

        for report in data.main_layout.reports.borrow().iter() {
            reports_column.add_child(
                ReportWidget::new(
                    report.id,
                    report.title.clone(),
                    report.game.clone(),
                    report.date.clone(),
                    report.time.clone(),
                )
                .build_ui(),
            );
        }

        Padding::new(
            5.0,
            Flex::column()
                .main_axis_alignment(MainAxisAlignment::Start)
                .must_fill_main_axis(true)
                .with_flex_child(reports_column, 1.0)
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
                            .on_click(MainLayout::on_open_first_page_clicked)
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
                                            })
                                            .on_click(MainLayout::on_prev_page_clicked),
                                        )
                                        .with_child(
                                            Label::new(|data: &ApplicationState, _env: &_| {
                                                data.main_layout.current_page.to_string()
                                            })
                                            .with_text_size(TEXT_SIZE),
                                        )
                                        .with_child(
                                            Button::from_label(
                                                Label::new(">").with_text_size(TEXT_SIZE),
                                            )
                                            .on_click(MainLayout::on_next_page_clicked),
                                        ),
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
    fn query_reports(&self, data: &ApplicationState) -> Result<Vec<ReportSummary>, ()> {
        let result = data
            .net_service
            .lock()
            .unwrap()
            .query_reports(self.current_page, REPORT_COUNT_PER_PAGE);

        if let Err(app_error) = result {
            data.logger_service
                .lock()
                .unwrap()
                .log(&app_error.to_string());
            return Err(());
        }

        Ok(result.unwrap())
    }
    fn on_open_first_page_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        let result = data
            .net_service
            .lock()
            .unwrap()
            .query_reports(1, REPORT_COUNT_PER_PAGE);

        if let Err(app_error) = result {
            data.logger_service
                .lock()
                .unwrap()
                .log(&app_error.to_string());
            return;
        }

        *data.main_layout.reports.borrow_mut() = result.unwrap();

        data.main_layout.current_page = 1;
    }
    fn on_prev_page_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        let result = data
            .net_service
            .lock()
            .unwrap()
            .query_reports(data.main_layout.current_page - 1, REPORT_COUNT_PER_PAGE);

        if let Err(app_error) = result {
            data.logger_service
                .lock()
                .unwrap()
                .log(&app_error.to_string());
            return;
        }

        *data.main_layout.reports.borrow_mut() = result.unwrap();

        data.main_layout.current_page -= 1;
    }
    fn on_next_page_clicked(_ctx: &mut EventCtx, data: &mut ApplicationState, _env: &Env) {
        let result = data
            .net_service
            .lock()
            .unwrap()
            .query_reports(data.main_layout.current_page + 1, REPORT_COUNT_PER_PAGE);

        if let Err(app_error) = result {
            data.logger_service
                .lock()
                .unwrap()
                .log(&app_error.to_string());
            return;
        }

        *data.main_layout.reports.borrow_mut() = result.unwrap();

        data.main_layout.current_page += 1;
    }
}
