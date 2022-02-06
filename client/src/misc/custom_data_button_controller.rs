use druid::widget::prelude::*;
use druid::widget::{Button, Controller};
use druid::{Selector, Target};

use crate::ApplicationState;

pub const CUSTOM_DATA_BUTTON_CLICKED: Selector<CustomButtonData> =
    Selector::new("custom_data_button_clicked");

#[derive(Clone)]
pub struct CustomButtonData {
    pub report_id: u64,
}

pub struct CustomDataButtonController {
    data: CustomButtonData,
}

impl CustomDataButtonController {
    pub fn new(data: CustomButtonData) -> Self {
        CustomDataButtonController { data }
    }
}

impl Controller<ApplicationState, Button<ApplicationState>> for CustomDataButtonController {
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
                    .submit_command(CUSTOM_DATA_BUTTON_CLICKED, self.data.clone(), Target::Auto)
                    .expect("ERROR: failed to submit CUSTOM_DATA_BUTTON_CLICKED command.");
                child.event(ctx, event, data, env)
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
