use druid::widget::prelude::*;
use druid::widget::{Button, Controller};
use druid::{Selector, Target};

use crate::ApplicationState;

pub const REPORT_ID_BUTTON_CLICKED: Selector<ReportIdButtonData> =
    Selector::new("report_id_button_clicked");

#[derive(Clone)]
pub struct ReportIdButtonData {
    pub report_id: u64,
}

pub struct ReportIdButtonController {
    data: ReportIdButtonData,
}

impl ReportIdButtonController {
    pub fn new(data: ReportIdButtonData) -> Self {
        Self { data }
    }
}

impl Controller<ApplicationState, Button<ApplicationState>> for ReportIdButtonController {
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
                    .submit_command(REPORT_ID_BUTTON_CLICKED, self.data.clone(), Target::Auto)
                    .expect("ERROR: failed to submit REPORT_ID_BUTTON_CLICKED command.");
                child.event(ctx, event, data, env)
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
