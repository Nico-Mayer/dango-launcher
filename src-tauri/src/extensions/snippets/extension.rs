//! The `snippets` built-in extension.
//!
//! A snippet is a name and a template. It is found in root search, and
//! confirming it renders the template and puts the result where the user was.
//! Everything about how that text gets there belongs to `crate::text`; this
//! decides what the text is and what the user sees.

use std::sync::Arc;

use async_trait::async_trait;

use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, Service,
    NAMED_ICON,
};
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{
    Action, EmptyState, FieldKind, Filtering, FormField, FormView, ListItem, ListView, View,
    ViewTree, PROTOCOL_VERSION,
};
use crate::search::{Candidate, RootProvider, Source};
use crate::templates::{Template, Values};
use crate::text::{TextError, TextTarget};

use super::service::{ExcludedApps, KeywordExpansion};
use super::store::{Kind, Record, Records};

pub const SNIPPETS_ID: &str = "dango.snippets";
pub const QUICKLINKS_ID: &str = "dango.quicklinks";

pub fn extension_id(kind: Kind) -> &'static str {
    match kind {
        Kind::Snippet => SNIPPETS_ID,
        Kind::Quicklink => QUICKLINKS_ID,
    }
}

fn icon(kind: Kind) -> String {
    match kind {
        Kind::Snippet => format!("{NAMED_ICON}clipboard-type"),
        Kind::Quicklink => format!("{NAMED_ICON}link"),
    }
}

pub const COMMAND_CREATE: &str = "create";
pub const COMMAND_SEARCH: &str = "search";

pub const ACTION_INSERT: &str = "insert";
pub const ACTION_COPY: &str = "copy";
pub const ACTION_EDIT: &str = "edit";
pub const ACTION_REMOVE: &str = "remove";
pub const ACTION_SAVE: &str = "save";

const FIELD_NAME: &str = "name";
const FIELD_BODY: &str = "body";
const FIELD_KEYWORD: &str = "keyword";
/// Prefix for the fields an argument form asks about, so an argument called
/// "name" cannot collide with the snippet's own name field.
const ARGUMENT_PREFIX: &str = "arg:";

pub struct SnippetsExtension {
    manifest: Manifest,
    records: Arc<Records>,
    /// `None` when key injection is unavailable, which makes inserting
    /// impossible but leaves everything else working.
    text: Option<Arc<dyn TextTarget>>,
    /// Only quicklinks need this, and only so the URL can be opened.
    opener: Option<Arc<dyn OpenUrl>>,
    /// The keyword-expansion service, only for snippets and only when text can
    /// be inserted. Its lifetime follows the extension's enabled state.
    services: Vec<Arc<dyn Service>>,
}

/// Opening a URL in whatever the user's default browser is. A trait so the
/// extension can be tested without a running Tauri application.
pub trait OpenUrl: Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;
}

impl SnippetsExtension {
    pub fn new(
        records: Arc<Records>,
        text: Option<Arc<dyn TextTarget>>,
        opener: Option<Arc<dyn OpenUrl>>,
        excluded: ExcludedApps,
    ) -> Self {
        let services: Vec<Arc<dyn Service>> = match (records.kind(), &text) {
            (Kind::Snippet, Some(text)) => vec![Arc::new(KeywordExpansion::new(
                records.clone(),
                text.clone(),
                excluded,
            ))],
            _ => Vec::new(),
        };
        Self {
            manifest: manifest(records.kind()),
            records,
            text,
            opener,
            services,
        }
    }

    fn kind(&self) -> Kind {
        self.records.kind()
    }

    /// The values a render needs. The clipboard and the selection are read only
    /// when the template actually asks for them, because reading a selection
    /// costs a clipboard round trip in the applications that do not answer the
    /// accessibility API.
    fn values(&self, template: &Template, mut arguments: FormValues) -> Values {
        // `query` is reserved, so it travels in its own slot rather than as one
        // of the arguments, even though the user is asked for it the same way.
        let query = arguments.remove("query");
        let mut values = Values {
            arguments,
            query,
            ..Default::default()
        };
        let Some(text) = &self.text else {
            return values;
        };
        if template.uses("clipboard") {
            values.clipboard = text.clipboard_text();
        }
        if template.uses("selection") {
            values.selection = text.selection().ok().flatten();
        }
        values
    }

    fn render(&self, record: &Record, arguments: FormValues) -> Result<Rendered, ActionOutcome> {
        let template = Template::parse(&record.body)
            .map_err(|error| ActionOutcome::Failed(error.to_string()))?;
        let values = self.values(&template, arguments);
        let rendered = template
            .render(&values)
            .map_err(|error| ActionOutcome::Failed(error.to_string()))?;
        Ok(Rendered {
            text: rendered.text,
            caret: rendered.caret,
        })
    }

    fn insert(&self, record: &Record, arguments: FormValues) -> ActionOutcome {
        let rendered = match self.render(record, arguments) {
            Ok(rendered) => rendered,
            Err(outcome) => return outcome,
        };
        let Some(text) = &self.text else {
            return ActionOutcome::Failed(TextError::PermissionMissing.to_string());
        };
        match text.insert(&rendered.text, rendered.caret) {
            Ok(()) => ActionOutcome::Done,
            Err(error) => ActionOutcome::Failed(error.to_string()),
        }
    }

    /// A quicklink's URL is rendered with every value encoded, so a query
    /// containing `&` or `?` cannot add a parameter or a fragment of its own.
    fn open(&self, record: &Record, arguments: FormValues) -> ActionOutcome {
        let template = match Template::parse(&record.body) {
            Ok(template) => template,
            Err(error) => return ActionOutcome::Failed(error.to_string()),
        };
        let values = self.values(&template, arguments);
        let url = match template.render_url(&values) {
            Ok(rendered) => rendered.text,
            Err(error) => return ActionOutcome::Failed(error.to_string()),
        };
        let Some(opener) = &self.opener else {
            return ActionOutcome::Failed("Dango can't open URLs on this platform.".into());
        };
        match opener.open(&url) {
            Ok(()) => ActionOutcome::Done,
            Err(error) => ActionOutcome::Failed(error),
        }
    }

    fn list_tree(&self) -> ViewTree {
        let items = match self.records.all() {
            Ok(records) => records,
            Err(error) => return failure_tree(&error.to_string(), self.records.kind()),
        };
        list_tree(&items, self.records.kind())
    }
}

pub struct Rendered {
    pub text: String,
    pub caret: Option<usize>,
}

impl Extension for SnippetsExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(Arc::new(SnippetProvider {
            records: self.records.clone(),
            kind: self.kind(),
        }))
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        self.services.clone()
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        match command_id {
            COMMAND_CREATE => Some(Arc::new(ShowForm(None, self.kind()))),
            COMMAND_SEARCH => Some(Arc::new(ShowList(self.records.clone()))),
            _ => None,
        }
    }

    fn perform_action(&self, item_id: &str, action_id: &str, values: &FormValues) -> ActionOutcome {
        match action_id {
            ACTION_SAVE => {
                let name = values.get(FIELD_NAME).map(String::as_str).unwrap_or("");
                let body = values.get(FIELD_BODY).map(String::as_str).unwrap_or("");
                // Absent for a quicklink form, which has no keyword field.
                let keyword = values.get(FIELD_KEYWORD).map(String::as_str);
                let saved = if item_id.is_empty() {
                    self.records.create(name, body, keyword).map(|_| ())
                } else {
                    self.records.update(item_id, name, body, keyword)
                };
                match saved {
                    Ok(()) => ActionOutcome::Replaced(Box::new(self.list_tree())),
                    Err(error) => ActionOutcome::Failed(error.to_string()),
                }
            }
            ACTION_EDIT => match self.records.get(item_id) {
                Ok(record) => {
                    ActionOutcome::Replaced(Box::new(form_tree(Some(&record), self.records.kind())))
                }
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            ACTION_REMOVE => match self.records.remove(item_id) {
                Ok(()) => ActionOutcome::Removed(Box::new(self.list_tree())),
                Err(error) => ActionOutcome::Failed(error.to_string()),
            },
            ACTION_COPY | ACTION_INSERT => {
                let record = match self.records.get(item_id) {
                    Ok(record) => record,
                    Err(error) => return ActionOutcome::Failed(error.to_string()),
                };
                let arguments = strip_prefix(values);

                // Asking has to happen before rendering, and only once: a form
                // that came back with values is not asked again.
                if arguments.is_empty() {
                    match Template::parse(&record.body) {
                        Ok(template) if !template.prompts().is_empty() => {
                            return ActionOutcome::Replaced(Box::new(argument_tree(
                                &record, &template, action_id,
                            )));
                        }
                        Ok(_) => {}
                        Err(error) => return ActionOutcome::Failed(error.to_string()),
                    }
                }

                match (action_id, self.kind()) {
                    (ACTION_COPY, _) => match self.render(&record, arguments) {
                        Ok(rendered) => ActionOutcome::CopyToClipboard(rendered.text),
                        Err(outcome) => outcome,
                    },
                    (_, Kind::Quicklink) => self.open(&record, arguments),
                    (_, Kind::Snippet) => self.insert(&record, arguments),
                }
            }
            _ => ActionOutcome::Failed("That action isn't available.".into()),
        }
    }
}

/// Argument fields are prefixed on the way out and stripped on the way back, so
/// a snippet argument called `name` cannot be mistaken for the snippet's own
/// name field.
fn strip_prefix(values: &FormValues) -> FormValues {
    values
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix(ARGUMENT_PREFIX)
                .map(|name| (name.to_string(), value.clone()))
        })
        .collect()
}

struct SnippetProvider {
    records: Arc<Records>,
    kind: Kind,
}

#[async_trait]
impl RootProvider for SnippetProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        self.records
            .all()
            .unwrap_or_default()
            .into_iter()
            .map(|record| Candidate {
                extension_id: extension_id(self.kind).into(),
                subtitle: Some(one_line(&record.body)),
                icon: Some(icon(self.kind)),
                title: record.name,
                id: record.id,
                keywords: vec![],
                alias: None,
                source: Source::RootItem,
                actions: item_actions(self.kind),
                match_positions: vec![],
            })
            .collect()
    }
}

fn item_actions(kind: Kind) -> Vec<Action> {
    let primary = match kind {
        Kind::Snippet => "Paste",
        Kind::Quicklink => "Open",
    };
    vec![
        Action {
            id: ACTION_INSERT.into(),
            title: primary.into(),
            shortcut: None,
        },
        Action {
            id: ACTION_COPY.into(),
            title: "Copy".into(),
            shortcut: None,
        },
        Action {
            id: ACTION_EDIT.into(),
            title: "Edit".into(),
            shortcut: None,
        },
        Action {
            id: ACTION_REMOVE.into(),
            title: "Delete".into(),
            shortcut: None,
        },
    ]
}

/// Collapses whitespace so a multi-line snippet still reads as one row, the way
/// the clipboard history shows a multi-line copy.
fn one_line(body: &str) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 80 {
        collapsed.chars().take(79).chain(['…']).collect()
    } else {
        collapsed
    }
}

struct ShowForm(Option<Record>, Kind);

impl Command for ShowForm {
    fn invoke(&self, ctx: &InvocationContext) {
        ctx.push_view(form_tree(self.0.as_ref(), self.1));
    }
}

struct ShowList(Arc<Records>);

impl Command for ShowList {
    fn invoke(&self, ctx: &InvocationContext) {
        match self.0.all() {
            Ok(records) => ctx.push_view(list_tree(&records, self.0.kind())),
            Err(error) => ctx.fail(error.to_string()),
        }
    }
}

fn form_tree(record: Option<&Record>, kind: Kind) -> ViewTree {
    let (body_label, save_title) = match kind {
        Kind::Snippet => ("Text", "Save snippet"),
        Kind::Quicklink => ("URL", "Save quicklink"),
    };
    let mut fields = vec![
        FormField {
            id: FIELD_NAME.into(),
            label: "Name".into(),
            kind: FieldKind::Text,
            value: record.map(|record| record.name.clone()),
        },
        FormField {
            id: FIELD_BODY.into(),
            label: body_label.into(),
            // A template field, so the form shows what it will ask for
            // while the user is still editing it.
            kind: FieldKind::Template,
            value: record.map(|record| record.body.clone()),
        },
    ];
    // Only a snippet expands from a keyword; a quicklink is not typed mid-line.
    if kind == Kind::Snippet {
        fields.push(FormField {
            id: FIELD_KEYWORD.into(),
            label: "Keyword (optional)".into(),
            kind: FieldKind::Text,
            value: record.and_then(|record| record.keyword.clone()),
        });
    }
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Form(FormView {
            item_id: record.map(|record| record.id.clone()),
            fields,
            actions: vec![Action {
                id: ACTION_SAVE.into(),
                title: save_title.into(),
                shortcut: None,
            }],
        }),
    }
}

fn argument_tree(record: &Record, template: &Template, action_id: &str) -> ViewTree {
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::Form(FormView {
            item_id: Some(record.id.clone()),
            fields: template
                .prompts()
                .iter()
                .map(|name| FormField {
                    id: format!("{ARGUMENT_PREFIX}{name}"),
                    label: name.clone(),
                    kind: FieldKind::Text,
                    value: None,
                })
                .collect(),
            actions: vec![Action {
                id: action_id.into(),
                title: record.name.clone(),
                shortcut: None,
            }],
        }),
    }
}

fn list_tree(records: &[Record], kind: Kind) -> ViewTree {
    let (title, description) = match kind {
        Kind::Snippet => (
            "No snippets yet",
            "Create one to keep text you type again and again.",
        ),
        Kind::Quicklink => (
            "No quicklinks yet",
            "Create one to reach a URL you visit often by name.",
        ),
    };
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::List(ListView {
            filtering: Filtering::Launcher,
            loading: false,
            empty_state: Some(EmptyState {
                title: title.into(),
                description: Some(description.into()),
            }),
            items: records
                .iter()
                .map(|record| ListItem {
                    id: record.id.clone(),
                    title: record.name.clone(),
                    subtitle: Some(one_line(&record.body)),
                    icon: Some(format!("{NAMED_ICON}clipboard-type")),
                    actions: item_actions(kind),
                })
                .collect(),
        }),
    }
}

fn failure_tree(message: &str, kind: Kind) -> ViewTree {
    let title = match kind {
        Kind::Snippet => "Couldn't read your snippets",
        Kind::Quicklink => "Couldn't read your quicklinks",
    };
    ViewTree {
        protocol_version: PROTOCOL_VERSION,
        view: View::List(ListView {
            filtering: Filtering::Launcher,
            loading: false,
            empty_state: Some(EmptyState {
                title: title.into(),
                description: Some(message.into()),
            }),
            items: vec![],
        }),
    }
}

fn manifest(kind: Kind) -> Manifest {
    let (name, create, search, keyword) = match kind {
        Kind::Snippet => ("Snippets", "Create Snippet", "Search Snippets", "snippet"),
        Kind::Quicklink => (
            "Quicklinks",
            "Create Quicklink",
            "Search Quicklinks",
            "quicklink",
        ),
    };
    Manifest {
        manifest_version: 1,
        id: extension_id(kind).into(),
        name: name.into(),
        icon: Some(icon(kind)),
        commands: vec![
            CommandDecl {
                id: COMMAND_CREATE.into(),
                title: create.into(),
                mode: InvocationMode::View,
                subtitle: None,
                icon: Some(icon(kind)),
                keywords: vec!["new".into(), keyword.into()],
                alias: None,
            },
            CommandDecl {
                id: COMMAND_SEARCH.into(),
                title: search.into(),
                mode: InvocationMode::View,
                subtitle: None,
                icon: Some(icon(kind)),
                keywords: vec![keyword.into()],
                alias: None,
            },
        ],
        preferences: vec![],
        root_items: true,
        services: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeTarget {
        inserted: Mutex<Vec<(String, Option<usize>)>>,
        clipboard: Option<String>,
        selection: Option<String>,
        fails: bool,
    }

    impl FakeTarget {
        fn working() -> Arc<Self> {
            Arc::new(Self::default())
        }
        fn inserted(&self) -> Vec<(String, Option<usize>)> {
            self.inserted.lock().unwrap().clone()
        }
    }

    impl TextTarget for FakeTarget {
        fn insert(&self, text: &str, caret: Option<usize>) -> Result<(), TextError> {
            if self.fails {
                return Err(TextError::PermissionMissing);
            }
            self.inserted
                .lock()
                .unwrap()
                .push((text.to_string(), caret));
            Ok(())
        }
        fn expand(
            &self,
            _backspaces: usize,
            text: &str,
            caret: Option<usize>,
        ) -> Result<(), TextError> {
            self.insert(text, caret)
        }
        fn paste_content(
            &self,
            _content: &crate::extensions::clipboard::Content,
        ) -> Result<(), TextError> {
            unreachable!("a snippet never pastes clipboard content")
        }
        fn selection(&self) -> Result<Option<String>, TextError> {
            Ok(self.selection.clone())
        }
        fn clipboard_text(&self) -> Option<String> {
            self.clipboard.clone()
        }
    }

    #[derive(Default)]
    struct FakeOpener(Mutex<Vec<String>>);

    impl FakeOpener {
        fn opened(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    impl OpenUrl for FakeOpener {
        fn open(&self, url: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(url.to_string());
            Ok(())
        }
    }

    struct Fixture {
        extension: SnippetsExtension,
        records: Arc<Records>,
        target: Arc<FakeTarget>,
        opener: Arc<FakeOpener>,
    }

    fn fixture(kind: Kind) -> Fixture {
        fixture_with(kind, FakeTarget::working())
    }

    fn fixture_with(kind: Kind, target: Arc<FakeTarget>) -> Fixture {
        let dir = std::env::temp_dir().join(format!("dango-snip-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let records = Records::open(&dir, kind).0;
        let opener = Arc::new(FakeOpener::default());
        Fixture {
            extension: SnippetsExtension::new(
                records.clone(),
                Some(target.clone()),
                Some(opener.clone()),
                Arc::new(Vec::new),
            ),
            records,
            target,
            opener,
        }
    }

    fn no_values() -> FormValues {
        FormValues::new()
    }

    fn values(pairs: &[(&str, &str)]) -> FormValues {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn items(tree: &ViewTree) -> Vec<ListItem> {
        match &tree.view {
            View::List(list) => list.items.clone(),
            _ => panic!("expected a list"),
        }
    }

    fn fields(tree: &ViewTree) -> Vec<FormField> {
        match &tree.view {
            View::Form(form) => form.fields.clone(),
            _ => panic!("expected a form"),
        }
    }

    #[test]
    fn the_two_kinds_do_not_share_an_identity() {
        assert_ne!(extension_id(Kind::Snippet), extension_id(Kind::Quicklink));
        assert_eq!(fixture(Kind::Snippet).extension.manifest().id, SNIPPETS_ID);
        assert_eq!(
            fixture(Kind::Quicklink).extension.manifest().id,
            QUICKLINKS_ID
        );
    }

    #[test]
    fn both_manifests_validate() {
        for kind in [Kind::Snippet, Kind::Quicklink] {
            assert!(fixture(kind).extension.manifest().validate().is_ok());
        }
    }

    #[tokio::test]
    async fn snippets_are_offered_in_root_search() {
        let f = fixture(Kind::Snippet);
        f.records
            .create("Signature", "Best,\n  Nico", None)
            .unwrap();

        let provider = f.extension.root_provider().unwrap();
        let candidates = provider.items(String::new()).await;

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Signature");
        assert_eq!(candidates[0].extension_id, SNIPPETS_ID);
        assert_eq!(
            candidates[0].subtitle.as_deref(),
            Some("Best, Nico"),
            "a multi-line snippet still reads as one row"
        );
    }

    #[tokio::test]
    async fn a_long_snippet_is_cut_rather_than_wrapped() {
        let f = fixture(Kind::Snippet);
        f.records.create("Long", &"word ".repeat(40), None).unwrap();

        let candidates = f
            .extension
            .root_provider()
            .unwrap()
            .items(String::new())
            .await;
        let subtitle = candidates[0].subtitle.clone().unwrap();
        assert_eq!(subtitle.chars().count(), 80);
        assert!(subtitle.ends_with('…'));
    }

    #[tokio::test]
    async fn the_provider_answers_within_the_budget_with_five_hundred_snippets() {
        let f = fixture(Kind::Snippet);
        for i in 0..500 {
            f.records
                .create(&format!("snippet {i}"), "body", None)
                .unwrap();
        }

        let provider = f.extension.root_provider().unwrap();
        let start = std::time::Instant::now();
        let candidates = provider.items(String::new()).await;
        let elapsed = start.elapsed();

        assert_eq!(candidates.len(), 500);
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "answering took {elapsed:?}, the provider budget is 50ms"
        );
    }

    #[test]
    fn confirming_a_snippet_inserts_its_rendered_text() {
        let f = fixture(Kind::Snippet);
        let id = f
            .records
            .create("Date", "Today is {{ date }}.", None)
            .unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(outcome, ActionOutcome::Done);
        let inserted = f.target.inserted();
        assert_eq!(inserted.len(), 1);
        assert!(inserted[0].0.starts_with("Today is 20"));
        assert_eq!(inserted[0].1, None);
    }

    #[test]
    fn a_snippet_using_the_clipboard_gets_it_without_asking() {
        let target = Arc::new(FakeTarget {
            clipboard: Some("copied text".into()),
            ..Default::default()
        });
        let f = fixture_with(Kind::Snippet, target);
        let id = f.records.create("Wrap", "> {{ clipboard }}", None).unwrap();

        f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(f.target.inserted()[0].0, "> copied text");
    }

    #[test]
    fn a_snippet_using_the_selection_gets_it_without_asking() {
        let target = Arc::new(FakeTarget {
            selection: Some("chosen text".into()),
            ..Default::default()
        });
        let f = fixture_with(Kind::Snippet, target);
        let id = f
            .records
            .create("Quote", "\"{{ selection }}\"", None)
            .unwrap();

        f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(f.target.inserted()[0].0, "\"chosen text\"");
    }

    #[test]
    fn a_caret_position_survives_to_the_insertion() {
        let f = fixture(Kind::Snippet);
        let id = f
            .records
            .create("Tag", "<b>{{ cursor }}</b>", None)
            .unwrap();

        f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(f.target.inserted()[0], ("<b></b>".to_string(), Some(3)));
    }

    #[test]
    fn a_snippet_with_arguments_asks_before_it_inserts() {
        let f = fixture(Kind::Snippet);
        let id = f
            .records
            .create("Greeting", "Dear {{ name }}, about {{ topic }}.", None)
            .unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        let ActionOutcome::Replaced(tree) = outcome else {
            panic!("expected a form, got {outcome:?}");
        };
        let asked: Vec<_> = fields(&tree).into_iter().map(|field| field.label).collect();
        assert_eq!(asked, vec!["name", "topic"], "in template order");
        assert!(
            f.target.inserted().is_empty(),
            "nothing is inserted before the user answers"
        );
    }

    #[test]
    fn submitting_the_arguments_inserts_the_rendered_text() {
        let f = fixture(Kind::Snippet);
        let id = f
            .records
            .create("Greeting", "Dear {{ name }}, about {{ topic }}.", None)
            .unwrap();
        let ActionOutcome::Replaced(tree) =
            f.extension.perform_action(&id, ACTION_INSERT, &no_values())
        else {
            panic!("expected the argument form");
        };
        let View::Form(form) = tree.view else {
            panic!("expected the argument form");
        };

        let outcome = f.extension.perform_action(
            form.item_id.as_deref().unwrap(),
            ACTION_INSERT,
            &values(&[("arg:name", "Nico"), ("arg:topic", "M3")]),
        );

        assert_eq!(outcome, ActionOutcome::Done);
        assert_eq!(f.target.inserted()[0].0, "Dear Nico, about M3.");
    }

    #[test]
    fn an_argument_named_like_the_form_field_does_not_collide() {
        let f = fixture(Kind::Snippet);
        let id = f.records.create("Hello", "Hi {{ name }}", None).unwrap();

        // `name` is also the id of the create form's own name field.
        let outcome =
            f.extension
                .perform_action(&id, ACTION_INSERT, &values(&[("arg:name", "Nico")]));

        assert_eq!(outcome, ActionOutcome::Done);
        assert_eq!(f.target.inserted()[0].0, "Hi Nico");
    }

    #[test]
    fn an_insertion_that_cannot_happen_says_so_and_inserts_nothing() {
        let target = Arc::new(FakeTarget {
            fails: true,
            ..Default::default()
        });
        let f = fixture_with(Kind::Snippet, target);
        let id = f.records.create("Signature", "Best, Nico", None).unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert!(matches!(outcome, ActionOutcome::Failed(_)));
        assert!(f.target.inserted().is_empty());
    }

    #[test]
    fn a_snippet_can_be_copied_instead_of_inserted() {
        let f = fixture(Kind::Snippet);
        let id = f.records.create("Signature", "Best, Nico", None).unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_COPY, &no_values());

        assert_eq!(outcome, ActionOutcome::CopyToClipboard("Best, Nico".into()));
        assert!(f.target.inserted().is_empty());
    }

    #[test]
    fn removing_a_snippet_leaves_the_list_open() {
        let f = fixture(Kind::Snippet);
        let id = f.records.create("Signature", "Best, Nico", None).unwrap();
        f.records.create("Other", "text", None).unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_REMOVE, &no_values());

        let ActionOutcome::Removed(tree) = outcome else {
            panic!("removing must refresh the surface the user was on");
        };
        assert_eq!(items(&tree).len(), 1);
    }

    #[test]
    fn saving_a_new_snippet_stores_it_and_shows_the_list() {
        let f = fixture(Kind::Snippet);

        let outcome = f.extension.perform_action(
            "",
            ACTION_SAVE,
            &values(&[("name", "Signature"), ("body", "Best, Nico")]),
        );

        assert!(matches!(outcome, ActionOutcome::Replaced(_)));
        let stored = f.records.all().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].name, "Signature");
    }

    #[test]
    fn saving_over_an_existing_snippet_edits_it() {
        let f = fixture(Kind::Snippet);
        let id = f.records.create("Signature", "Best, Nico", None).unwrap();

        f.extension.perform_action(
            &id,
            ACTION_SAVE,
            &values(&[("name", "Sign-off"), ("body", "Regards")]),
        );

        let stored = f.records.all().unwrap();
        assert_eq!(stored.len(), 1, "an edit is not a second snippet");
        assert_eq!(stored[0].name, "Sign-off");
    }

    #[test]
    fn saving_something_unusable_says_what_is_wrong_and_stores_nothing() {
        let f = fixture(Kind::Snippet);

        let outcome =
            f.extension
                .perform_action("", ACTION_SAVE, &values(&[("name", ""), ("body", "text")]));

        assert!(matches!(outcome, ActionOutcome::Failed(_)));
        assert!(f.records.all().unwrap().is_empty());
    }

    #[test]
    fn editing_offers_the_snippet_as_a_template_field() {
        let f = fixture(Kind::Snippet);
        let id = f.records.create("Signature", "Best, Nico", None).unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_EDIT, &no_values());

        let ActionOutcome::Replaced(tree) = outcome else {
            panic!("expected the edit form");
        };
        let View::Form(form) = &tree.view else {
            panic!("expected the edit form");
        };
        assert_eq!(form.item_id.as_deref(), Some(id.as_str()));
        let fields = fields(&tree);
        assert_eq!(fields[0].value.as_deref(), Some("Signature"));
        assert_eq!(fields[1].value.as_deref(), Some("Best, Nico"));
        assert_eq!(
            fields[1].kind,
            FieldKind::Template,
            "so the form shows what it will ask for"
        );
    }

    #[test]
    fn the_empty_state_explains_itself() {
        for kind in [Kind::Snippet, Kind::Quicklink] {
            let f = fixture(kind);
            let tree = f.extension.list_tree();
            let View::List(list) = &tree.view else {
                panic!("expected a list");
            };
            let empty = list.empty_state.clone().expect("an empty state");
            assert!(list.items.is_empty());
            assert!(empty.description.is_some());
        }
    }

    #[test]
    fn confirming_a_quicklink_opens_its_url() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(outcome, ActionOutcome::Done);
        assert_eq!(f.opener.opened(), vec!["https://example.com/docs"]);
        assert!(f.target.inserted().is_empty(), "a quicklink is not pasted");
    }

    #[test]
    fn a_quicklink_query_is_encoded_so_it_cannot_change_the_url() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .create("Search", "https://example.com/s?q={{ query }}&safe=1", None)
            .unwrap();

        f.extension.perform_action(
            &id,
            ACTION_INSERT,
            &values(&[("arg:query", "rust lang & more?")]),
        );

        let opened = f.opener.opened();
        assert!(opened[0].contains("rust%20lang"), "got {}", opened[0]);
        assert!(opened[0].ends_with("&safe=1"), "got {}", opened[0]);
        assert_eq!(opened[0].matches('&').count(), 1, "got {}", opened[0]);
    }

    #[test]
    fn a_quicklink_that_takes_a_query_asks_for_it() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .create("Search", "https://example.com/s?q={{ query }}", None)
            .unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        let ActionOutcome::Replaced(tree) = outcome else {
            panic!("expected a form asking for the query");
        };
        assert_eq!(fields(&tree).len(), 1);
        assert!(f.opener.opened().is_empty());
    }

    #[test]
    fn a_quicklink_using_the_clipboard_is_not_asked_about() {
        let target = Arc::new(FakeTarget {
            clipboard: Some("dango".into()),
            ..Default::default()
        });
        let f = fixture_with(Kind::Quicklink, target);
        let id = f
            .records
            .create("Search", "https://example.com/s?q={{ clipboard }}", None)
            .unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(outcome, ActionOutcome::Done);
        assert_eq!(f.opener.opened(), vec!["https://example.com/s?q=dango"]);
    }

    #[test]
    fn a_quicklinks_url_can_be_copied_instead_of_opened() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();

        let outcome = f.extension.perform_action(&id, ACTION_COPY, &no_values());

        assert_eq!(
            outcome,
            ActionOutcome::CopyToClipboard("https://example.com/docs".into())
        );
        assert!(f.opener.opened().is_empty());
    }

    #[tokio::test]
    async fn a_quicklinks_primary_action_reads_as_open() {
        let f = fixture(Kind::Quicklink);
        f.records
            .create("Docs", "https://example.com", None)
            .unwrap();

        let candidates = f
            .extension
            .root_provider()
            .unwrap()
            .items(String::new())
            .await;

        assert_eq!(candidates[0].actions[0].title, "Open");
    }

    #[test]
    fn acting_on_something_that_is_gone_says_so() {
        let f = fixture(Kind::Snippet);
        let outcome = f
            .extension
            .perform_action("nope", ACTION_INSERT, &no_values());
        assert!(matches!(outcome, ActionOutcome::Failed(_)));
    }
}
