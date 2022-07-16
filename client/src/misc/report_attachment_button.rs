use druid::widget::prelude::*;
use druid::widget::{Button, Controller};
use druid::{Selector, Target};

use crate::ApplicationState;

pub const REPORT_ATTACHMENT_BUTTON_CLICKED: Selector<ReportAttachmentButtonData> =
    Selector::new("report_attachment_button_clicked");

#[derive(Clone)]
pub struct ReportAttachmentButtonData {
    pub attachment_id: usize,
    pub attachment_file_name: String,
}

pub struct ReportAttachmentButtonController {
    data: ReportAttachmentButtonData,
}

impl ReportAttachmentButtonController {
    pub fn new(data: ReportAttachmentButtonData) -> Self {
        Self { data }
    }
}

impl Controller<ApplicationState, Button<ApplicationState>> for ReportAttachmentButtonController {
    fn event(
        &mut self,
        child: &mut Button<ApplicationState>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut ApplicationState,
        env: &Env,
    ) {
        match event {
            Event::MouseUp(_) => {
                ctx.get_external_handle()
                    .submit_command(
                        REPORT_ATTACHMENT_BUTTON_CLICKED,
                        self.data.clone(),
                        Target::Auto,
                    )
                    .expect("ERROR: failed to submit REPORT_ATTACHMENT_BUTTON_CLICKED command.");
                child.event(ctx, event, data, env)
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
