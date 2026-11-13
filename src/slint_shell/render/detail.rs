//! The `DetailData` global: an image, a body, and its metadata.

use std::rc::Rc;

use slint::{ComponentHandle, Image, Model, ModelRc, StyledText, VecModel};

use crate::slint_shell::generated::{DetailData, IconKind, LauncherWindow, MetaData};
use crate::slint_shell::icons::Icons;
use crate::ui::snapshot::DetailBody;

/// The body last written, so an unchanged markdown body is not parsed again.
enum Body {
    Plain(String),
    Markdown(String),
}

impl Body {
    fn shows(&self, body: &DetailBody) -> bool {
        match (self, &body.text) {
            (Body::Plain(shown), Some(text)) => shown == text,
            (Body::Markdown(shown), None) => *shown == body.markdown,
            _ => false,
        }
    }
}

pub struct Writer {
    metadata: Rc<VecModel<MetaData>>,
    body: Option<Body>,
    ui: slint::Weak<LauncherWindow>,
}

impl Writer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let metadata = Rc::new(VecModel::default());
        ui.global::<DetailData>()
            .set_metadata(ModelRc::from(metadata.clone()));
        Self {
            metadata,
            body: None,
            ui: ui.as_weak(),
        }
    }

    pub fn write(&mut self, body: &DetailBody, icons: &Icons) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let detail = ui.global::<DetailData>();
        let (image, kind) = super::icon(icons, body.image.as_deref());
        detail.set_has_image(kind != IconKind::None);
        detail.set_image_named(kind == IconKind::Named);
        detail.set_image(image);
        self.write_body(&detail, body);
        self.write_metadata(body);
    }

    fn write_body(&mut self, detail: &DetailData, body: &DetailBody) {
        if self.body.as_ref().is_some_and(|shown| shown.shows(body)) {
            return;
        }
        match &body.text {
            Some(text) => {
                detail.set_plain(true);
                detail.set_body(StyledText::default());
                detail.set_text(text.as_str().into());
                self.body = Some(Body::Plain(text.clone()));
            }
            None => {
                detail.set_plain(false);
                detail.set_body(
                    StyledText::from_markdown(&body.markdown)
                        .unwrap_or_else(|_| StyledText::from_plain_text(&body.markdown)),
                );
                detail.set_text(body.markdown.as_str().into());
                self.body = Some(Body::Markdown(body.markdown.clone()));
            }
        }
    }

    fn write_metadata(&self, body: &DetailBody) {
        let unchanged = self.metadata.row_count() == body.metadata.len()
            && self
                .metadata
                .iter()
                .zip(&body.metadata)
                .all(|(shown, pair)| shown.label == pair.label && shown.value == pair.value);
        if unchanged {
            return;
        }
        let metadata = body.metadata.iter().map(|pair| MetaData {
            label: pair.label.as_str().into(),
            value: pair.value.as_str().into(),
        });
        self.metadata.set_vec(metadata.collect::<Vec<_>>());
    }

    pub fn clear(&mut self) {
        self.body = None;
        if let Some(ui) = self.ui.upgrade() {
            let detail = ui.global::<DetailData>();
            detail.set_has_image(false);
            detail.set_image(Image::default());
        }
    }
}
