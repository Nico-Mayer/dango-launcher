//! The `DetailData` global: a body and its metadata.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, StyledText, VecModel};

use crate::slint_shell::generated::{DetailData, LauncherWindow, MetaData};
use crate::ui::snapshot::DetailBody;

pub struct Writer {
    metadata: Rc<VecModel<MetaData>>,
    ui: slint::Weak<LauncherWindow>,
}

impl Writer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let metadata = Rc::new(VecModel::default());
        ui.global::<DetailData>()
            .set_metadata(ModelRc::from(metadata.clone()));
        Self {
            metadata,
            ui: ui.as_weak(),
        }
    }

    pub fn write(&self, body: &DetailBody) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let detail = ui.global::<DetailData>();
        detail.set_body(StyledText::from_plain_text(&body.markdown));
        detail.set_text(body.markdown.as_str().into());
        let metadata = body.metadata.iter().map(|pair| MetaData {
            label: pair.label.as_str().into(),
            value: pair.value.as_str().into(),
        });
        self.metadata.set_vec(metadata.collect::<Vec<_>>());
    }
}
