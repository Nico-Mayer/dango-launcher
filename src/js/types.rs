//! `dango.d.ts`: editor types for extension authors, generated from Dango's
//! own types in a test and shipped as a checked-in file.

use std::path::Path;

pub const FILE_NAME: &str = "dango.d.ts";

const TYPES: &str = include_str!("../../assets/dango.d.ts");

/// Writes the types file into an existing extensions folder, unless it
/// already holds this version. A missing folder stays missing.
pub fn write(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let path = dir.join(FILE_NAME);
    if std::fs::read_to_string(&path).is_ok_and(|current| current == TYPES) {
        return;
    }
    if let Err(error) = std::fs::write(&path, TYPES) {
        crate::log::append(&format!("can't write {}: {error}", path.display()));
    }
}

#[cfg(test)]
mod tests {
    use ts_rs::{Config, TS};

    use super::*;
    use crate::js::root::RootItem;
    use crate::protocol::*;

    const UPDATE: &str = "DANGO_UPDATE_TYPES";

    fn generate() -> String {
        let cfg = Config::new();
        let mut decls = vec![
            ViewTree::decl(&cfg),
            View::decl(&cfg),
            ListView::decl(&cfg),
            GridView::decl(&cfg),
            GridItem::decl(&cfg),
            TileContent::decl(&cfg),
            Filter::decl(&cfg),
            Filtering::decl(&cfg),
            Section::decl(&cfg),
            ListItem::decl(&cfg),
            ItemDetail::decl(&cfg),
            Metadata::decl(&cfg),
            Accessory::decl(&cfg),
            Status::decl(&cfg),
            TagRole::decl(&cfg),
            DetailView::decl(&cfg),
            ProgressView::decl(&cfg),
            Step::decl(&cfg),
            FormView::decl(&cfg),
            FormField::decl(&cfg),
            FieldOption::decl(&cfg),
            FieldKind::decl(&cfg),
            EmptyState::decl(&cfg),
            Action::decl(&cfg),
            ActionStyle::decl(&cfg),
            Shortcut::decl(&cfg),
            Modifier::decl(&cfg),
            RootItem::decl(&cfg),
        ];
        decls.extend(crate::js::manifest::type_decls(&cfg));
        let generated = decls
            .into_iter()
            .map(authored)
            .collect::<Vec<_>>()
            .join("\n\n");
        include_str!("dango.d.ts.in").replace("{{GENERATED}}", &generated)
    }

    /// Extension code writes actions with handlers and no ids, and a view
    /// that searches with its handler; the prelude turns both into protocol.
    fn authored(decl: String) -> String {
        let decl = if decl.starts_with("type Action = ") {
            decl.replacen("id: string,", "id?: string, onAction?: ActionHandler,", 1)
        } else {
            decl
        };
        if decl.starts_with("type ListView = ") || decl.starts_with("type GridView = ") {
            return decl.replacen("= {", "= { onSearch?: SearchHandler,", 1);
        }
        decl
    }

    #[test]
    fn the_shipped_types_are_current() {
        let generated = generate();
        if std::env::var_os(UPDATE).is_some() {
            std::fs::write(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("assets")
                    .join(FILE_NAME),
                &generated,
            )
            .unwrap();
            return;
        }
        assert!(
            generated == TYPES,
            "assets/dango.d.ts is out of date. Run:\n  env {UPDATE}=1 cargo test the_shipped_types_are_current"
        );
    }

    #[test]
    fn authors_write_handlers_instead_of_ids() {
        assert!(TYPES.contains("onAction?: ActionHandler"));
        assert!(TYPES.contains("onSearch?: SearchHandler"));
        assert!(TYPES.contains("type Manifest = "));
        assert!(TYPES.contains("type RootItem = "));
    }

    #[test]
    fn the_file_is_written_only_into_an_existing_folder() {
        let dir = crate::js::load::tests::temp();
        let missing = dir.join("extensions");

        write(&missing);
        assert!(!missing.exists());

        std::fs::create_dir_all(&missing).unwrap();
        write(&missing);
        assert_eq!(
            std::fs::read_to_string(missing.join(FILE_NAME)).unwrap(),
            TYPES
        );
    }
}
