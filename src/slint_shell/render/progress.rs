//! The `ProgressData` global: one long job's title, bar, detail, and steps.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use super::list::slint_status;
use crate::slint_shell::generated::{LauncherWindow, ProgressData, StepData};
use crate::ui::snapshot::ProgressBody;

pub fn write(ui: &LauncherWindow, body: &ProgressBody) {
    let progress = ui.global::<ProgressData>();
    progress.set_title(body.title.as_str().into());
    progress.set_subtitle(body.subtitle.as_str().into());
    progress.set_value(body.value.map_or(-1.0, |value| value.clamp(0.0, 1.0)));
    progress.set_detail(body.detail.as_str().into());
    let steps: Vec<StepData> = body
        .steps
        .iter()
        .map(|step| StepData {
            title: step.title.as_str().into(),
            status: slint_status(step.status),
        })
        .collect();
    progress.set_steps(ModelRc::from(Rc::new(VecModel::from(steps))));
}
