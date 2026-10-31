//! The `DetailData` global: a body and its metadata.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, StyledText, VecModel};

use crate::slint_shell::generated::{DetailData, LauncherWindow, MetaData};

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

    pub fn write(&self, markdown: &str) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let detail = ui.global::<DetailData>();
        detail.set_body(StyledText::from_plain_text(markdown));
        detail.set_text(markdown.into());
        self.metadata.set_vec(Vec::new());
    }
}
