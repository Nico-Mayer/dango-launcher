//! The `FormData` global: the fields, which own their text while the user
//! edits.

use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::protocol;
use crate::slint_shell::generated::{FieldData, FieldKind, FormData, LauncherWindow, OptionData};
use crate::ui::snapshot::{Field, FormBody};

pub struct Writer {
    fields: Rc<VecModel<FieldData>>,
    revision: Option<u64>,
}

impl Writer {
    pub fn new(ui: &LauncherWindow) -> Self {
        let fields = Rc::new(VecModel::default());
        ui.global::<FormData>()
            .set_fields(ModelRc::from(fields.clone()));
        Self {
            fields,
            revision: None,
        }
    }

    pub fn forget(&mut self) {
        self.revision = None;
    }

    /// A new revision seeds the fields again; otherwise only their help and
    /// errors move.
    pub fn write(&mut self, form: &FormBody) {
        let fields: Vec<FieldData> = form.fields.iter().map(field).collect();
        if self.revision != Some(form.revision) || self.fields.row_count() != fields.len() {
            self.fields.set_vec(fields);
            self.revision = Some(form.revision);
            return;
        }
        for (index, field) in fields.into_iter().enumerate() {
            if self.fields.row_data(index).as_ref() != Some(&field) {
                self.fields.set_row_data(index, field);
            }
        }
    }
}

fn field(field: &Field) -> FieldData {
    FieldData {
        id: field.id.as_str().into(),
        label: field.label.as_str().into(),
        kind: match field.kind {
            protocol::FieldKind::Text => FieldKind::Text,
            protocol::FieldKind::Password => FieldKind::Password,
            protocol::FieldKind::Textarea => FieldKind::Textarea,
            protocol::FieldKind::Toggle => FieldKind::Toggle,
            protocol::FieldKind::Dropdown => FieldKind::Dropdown,
            protocol::FieldKind::Template => FieldKind::Template,
            protocol::FieldKind::Separator => FieldKind::Separator,
        },
        options: ModelRc::from(Rc::new(VecModel::from(
            field
                .options
                .iter()
                .map(|option| OptionData {
                    id: option.id.as_str().into(),
                    title: option.title.as_str().into(),
                })
                .collect::<Vec<_>>(),
        ))),
        current: field
            .options
            .iter()
            .position(|option| option.id == field.value)
            .unwrap_or(0) as i32,
        value: field.value.as_str().into(),
        help: field.help.clone().unwrap_or_default().into(),
        help_emphasized: field.help_emphasized,
        error: field.error.clone().unwrap_or_default().into(),
    }
}
