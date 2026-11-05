//! The `snippets` built-in extension.
//!
//! A snippet is a name and a template. It is found in root search, and
//! confirming it renders the template and puts the result where the user was.
//! Everything about how that text gets there belongs to `crate::text`; this
//! decides what the text is and what the user sees.

use std::sync::Arc;

use async_trait::async_trait;

use crate::browser::Browser;
use crate::extension::{
    ActionOutcome, CommandDecl, Extension, FormValues, InvocationMode, Manifest, Service,
    NAMED_ICON,
};
use crate::invocation::{Command, InvocationContext};
use crate::protocol::{
    Action, ActionStyle, EmptyState, FieldKind, FieldOption, FormField, FormView, ListItem,
    ListView, View, ViewTree, PROTOCOL_VERSION,
};
use crate::search::{Candidate, RootProvider, Source};
use crate::templates::{Rendered, Template, Values};
use crate::text::{TextError, TextTarget};

use super::favicons::{FaviconCache, FaviconService, FaviconSource, FaviconsEnabled};
use super::service::{ExcludedApps, KeywordExpansion};
use super::store::{Kind, Record, RecordError, Records};

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
const FIELD_BROWSER: &str = "browser";
const FIELD_FALLBACK: &str = "fallback";
/// Starts a web search row's id, which goes on to the record id and the typed
/// query, so the row carries its query to the action.
const WEB_SEARCH_PREFIX: &str = "web:";
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
    icons: Icons,
}

/// Opening a URL in whatever the user's default browser is. A trait so the
/// extension can be tested without opening a browser.
pub trait OpenUrl: Send + Sync {
    fn open(&self, url: &str) -> Result<(), String>;

    /// Opens in `browser` when it is a browser this platform can start, and in
    /// the default browser when it is not or fails to start.
    fn open_in(&self, url: &str, _browser: Option<&str>) -> Result<(), String> {
        self.open(url)
    }
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
            icons: Icons::default(),
        }
    }

    /// Adds the site icons: the cache the search path reads and the service that
    /// fills it. Only quicklinks have a URL to take an icon from.
    pub fn with_favicons(
        mut self,
        cache: Arc<FaviconCache>,
        source: Arc<dyn FaviconSource>,
        enabled: FaviconsEnabled,
    ) -> Self {
        if self.kind() != Kind::Quicklink {
            return self;
        }
        self.services.push(Arc::new(FaviconService::new(
            self.records.clone(),
            cache.clone(),
            source,
            enabled.clone(),
        )));
        self.icons = Icons {
            favicons: Some(cache),
            enabled: Some(enabled),
        };
        self
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
        template
            .render(&values)
            .map_err(|error| ActionOutcome::Failed(error.to_string()))
    }

    fn render_url(&self, record: &Record, arguments: FormValues) -> Result<String, ActionOutcome> {
        let template = Template::parse(&record.body)
            .map_err(|error| ActionOutcome::Failed(error.to_string()))?;
        let values = self.values(&template, arguments);
        template
            .render_url(&values)
            .map(|rendered| rendered.text)
            .map_err(|error| ActionOutcome::Failed(error.to_string()))
    }

    fn insert(&self, record: &Record, arguments: FormValues) -> ActionOutcome {
        let rendered = match self.render(record, arguments) {
            Ok(rendered) => rendered,
            Err(outcome) => return outcome,
        };
        let Some(text) = &self.text else {
            return ActionOutcome::Failed(TextError::Unavailable.to_string());
        };
        match text.insert(&rendered.text, rendered.caret) {
            Ok(()) => ActionOutcome::Done,
            Err(error) => ActionOutcome::Failed(error.to_string()),
        }
    }

    /// A quicklink's URL is rendered with every value encoded, so a query
    /// containing `&` or `?` cannot add a parameter or a fragment of its own.
    fn open(&self, record: &Record, arguments: FormValues) -> ActionOutcome {
        let url = match self.render_url(record, arguments) {
            Ok(url) => url,
            Err(outcome) => return outcome,
        };
        let Some(opener) = &self.opener else {
            return ActionOutcome::Failed("Dango can't open URLs on this platform".into());
        };
        match opener.open_in(&url, record.browser.as_deref()) {
            Ok(()) => ActionOutcome::Done,
            Err(error) => ActionOutcome::Failed(error),
        }
    }

    /// A web search asks for nothing but the query its id carries, so it goes
    /// straight to opening or copying.
    fn search(&self, web_search: &str, action_id: &str) -> ActionOutcome {
        let Some((id, query)) = web_search.split_once(':') else {
            return ActionOutcome::Failed(RecordError::Gone.to_string());
        };
        let record = match self.records.get(id) {
            Ok(record) => record,
            Err(error) => return ActionOutcome::Failed(error.to_string()),
        };
        let arguments = FormValues::from([("query".to_string(), query.to_string())]);
        match action_id {
            ACTION_INSERT => self.open(&record, arguments),
            ACTION_COPY => match self.render_url(&record, arguments) {
                Ok(url) => ActionOutcome::CopyToClipboard(url),
                Err(outcome) => outcome,
            },
            _ => ActionOutcome::Failed("That action isn't available".into()),
        }
    }

    fn list_tree(&self) -> ViewTree {
        let items = match self.records.all() {
            Ok(records) => records,
            Err(error) => return failure_tree(&error.to_string(), self.records.kind()),
        };
        list_tree(&items, &self.icons.sheet(&self.records))
    }
}

impl Extension for SnippetsExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(Arc::new(SnippetProvider {
            records: self.records.clone(),
            kind: self.kind(),
            icons: self.icons.clone(),
        }))
    }

    fn services(&self) -> Vec<Arc<dyn Service>> {
        self.services.clone()
    }

    fn command(&self, command_id: &str) -> Option<Arc<dyn Command>> {
        match command_id {
            COMMAND_CREATE => Some(Arc::new(ShowForm(None, self.kind()))),
            COMMAND_SEARCH => Some(Arc::new(ShowList(self.records.clone(), self.icons.clone()))),
            _ => None,
        }
    }

    fn perform_action(&self, item_id: &str, action_id: &str, values: &FormValues) -> ActionOutcome {
        if let Some(web_search) = item_id.strip_prefix(WEB_SEARCH_PREFIX) {
            return self.search(web_search, action_id);
        }
        match action_id {
            ACTION_SAVE => {
                let name = values.get(FIELD_NAME).map(String::as_str).unwrap_or("");
                let body = values.get(FIELD_BODY).map(String::as_str).unwrap_or("");
                // Absent for a quicklink form, which has no keyword field.
                let keyword = values.get(FIELD_KEYWORD).map(String::as_str);
                // Absent for a snippet form, which has no browser field.
                let browser = values.get(FIELD_BROWSER).map(String::as_str);
                let fallback = values.get(FIELD_FALLBACK).map(|value| value == "true");
                let id = (!item_id.is_empty()).then_some(item_id);
                match self
                    .records
                    .save(id, name, body, keyword, browser, fallback)
                {
                    Ok(_) => ActionOutcome::Replaced(Box::new(self.list_tree())),
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
            _ => ActionOutcome::Failed("That action isn't available".into()),
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

/// Where a record's icon comes from. Empty for snippets, which have no site to
/// take one from, and for a quicklink extension wired without a cache.
#[derive(Clone, Default)]
struct Icons {
    favicons: Option<Arc<FaviconCache>>,
    enabled: Option<FaviconsEnabled>,
}

impl Icons {
    /// Fixes what is in play for one list. The preference is read here rather
    /// than per record, because it is read from the config file.
    fn sheet<'a>(&'a self, records: &'a Records) -> Sheet<'a> {
        let wanted = self.enabled.as_ref().is_none_or(|enabled| enabled());
        Sheet {
            kind: records.kind(),
            records,
            favicons: wanted.then_some(self.favicons.as_deref()).flatten(),
        }
    }
}

/// The icon choice for the records of one list, made without a syscall so it can
/// run on the search path.
struct Sheet<'a> {
    kind: Kind,
    records: &'a Records,
    favicons: Option<&'a FaviconCache>,
}

impl Sheet<'_> {
    fn of(&self, record: &Record) -> String {
        self.favicons
            .and_then(|cache| cache.path(&self.records.host(&record.id)?))
            .unwrap_or_else(|| icon(self.kind))
    }
}

struct SnippetProvider {
    records: Arc<Records>,
    kind: Kind,
    icons: Icons,
}

#[async_trait]
impl RootProvider for SnippetProvider {
    async fn items(&self, query: String) -> Vec<Candidate> {
        let sheet = self.icons.sheet(&self.records);
        let mut candidates: Vec<Candidate> = self
            .records
            .all()
            .unwrap_or_default()
            .into_iter()
            .map(|record| Candidate {
                extension_id: extension_id(self.kind).into(),
                status: None,
                icon: Some(sheet.of(&record)),
                title: record.name,
                id: record.id,
                keywords: vec![],
                alias: None,
                source: Source::RootItem,
                actions: item_actions(self.kind),
                match_positions: vec![],
                suggested: false,
                pinned: false,
            })
            .collect();
        let query = query.trim();
        if self.kind == Kind::Quicklink && !query.is_empty() {
            candidates.extend(web_searches(&self.records, &sheet, query));
        }
        candidates
    }
}

/// An id with `:` in it, possible only in a hand-written file, is left out:
/// the row's id splits at the first `:` after the prefix.
fn web_searches(records: &Records, sheet: &Sheet, query: &str) -> Vec<Candidate> {
    records
        .in_file_order()
        .into_iter()
        .filter(|record| record.is_web_search() && !record.id.contains(':'))
        .map(|record| Candidate {
            extension_id: QUICKLINKS_ID.into(),
            id: format!("{WEB_SEARCH_PREFIX}{}:{query}", record.id),
            title: format!("Search {} for “{query}”", record.name),
            status: None,
            icon: Some(sheet.of(&record)),
            keywords: vec![],
            alias: None,
            source: Source::WebSearch,
            actions: vec![
                Action {
                    id: ACTION_INSERT.into(),
                    title: "Open".into(),
                    ..Default::default()
                },
                Action {
                    id: ACTION_COPY.into(),
                    title: "Copy".into(),
                    ..Default::default()
                },
            ],
            match_positions: vec![],
            suggested: false,
            pinned: false,
        })
        .collect()
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
            ..Default::default()
        },
        Action {
            id: ACTION_COPY.into(),
            title: "Copy".into(),
            shortcut: None,
            ..Default::default()
        },
        Action {
            id: ACTION_EDIT.into(),
            title: "Edit".into(),
            shortcut: None,
            ..Default::default()
        },
        Action {
            id: ACTION_REMOVE.into(),
            title: "Delete".into(),
            shortcut: None,
            style: ActionStyle::Danger,
            ..Default::default()
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

struct ShowList(Arc<Records>, Icons);

impl Command for ShowList {
    fn invoke(&self, ctx: &InvocationContext) {
        match self.0.all() {
            Ok(records) => ctx.push_view(list_tree(&records, &self.1.sheet(&self.0))),
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
            ..Default::default()
        },
        FormField {
            id: FIELD_BODY.into(),
            label: body_label.into(),
            // A template field, so the form shows what it will ask for
            // while the user is still editing it.
            kind: FieldKind::Template,
            value: record.map(|record| record.body.clone()),
            ..Default::default()
        },
    ];
    // Only a snippet expands from a keyword; a quicklink is not typed mid-line.
    if kind == Kind::Snippet {
        fields.push(FormField {
            id: FIELD_KEYWORD.into(),
            label: "Keyword (optional)".into(),
            kind: FieldKind::Text,
            value: record.and_then(|record| record.keyword.clone()),
            ..Default::default()
        });
    }
    if kind == Kind::Quicklink {
        fields.push(browser_field(record));
        fields.push(FormField {
            id: FIELD_FALLBACK.into(),
            label: "Use for web search".into(),
            kind: FieldKind::Toggle,
            value: Some(record.is_some_and(|record| record.fallback).to_string()),
            ..Default::default()
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
                ..Default::default()
            }],
        }),
    }
}

/// The stored id is the value even when this platform does not offer it: the
/// dropdown then shows "Default browser" but submits the id unchanged, so an
/// edit does not drop it.
fn browser_field(record: Option<&Record>) -> FormField {
    let default = FieldOption {
        id: Browser::DEFAULT_ID.into(),
        title: "Default browser".into(),
    };
    FormField {
        id: FIELD_BROWSER.into(),
        label: "Browser".into(),
        kind: FieldKind::Dropdown,
        value: record.and_then(|record| record.browser.clone()),
        options: std::iter::once(default)
            .chain(Browser::offered().map(|browser| FieldOption {
                id: browser.id().into(),
                title: browser.title().into(),
            }))
            .collect(),
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
                    ..Default::default()
                })
                .collect(),
            actions: vec![Action {
                id: action_id.into(),
                title: record.name.clone(),
                shortcut: None,
                ..Default::default()
            }],
        }),
    }
}

fn list_tree(records: &[Record], sheet: &Sheet) -> ViewTree {
    let (title, description) = match sheet.kind {
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
            loading: false,
            empty_state: Some(EmptyState {
                title: title.into(),
                description: Some(description.into()),
                ..Default::default()
            }),
            items: records
                .iter()
                .map(|record| ListItem {
                    id: record.id.clone(),
                    title: record.name.clone(),
                    subtitle: Some(one_line(&record.body)),
                    icon: Some(sheet.of(record)),
                    actions: item_actions(sheet.kind),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
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
            loading: false,
            empty_state: Some(EmptyState {
                title: title.into(),
                description: Some(message.into()),
                ..Default::default()
            }),
            items: vec![],
            ..Default::default()
        }),
    }
}

pub(crate) fn manifest(kind: Kind) -> Manifest {
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
                icon: Some(icon(kind)),
                keywords: vec!["new".into(), keyword.into()],
                alias: None,
                repeats: false,
            },
            CommandDecl {
                id: COMMAND_SEARCH.into(),
                title: search.into(),
                mode: InvocationMode::View,
                icon: Some(icon(kind)),
                keywords: vec![keyword.into()],
                alias: None,
                repeats: false,
            },
        ],
        root_items: true,
        // Both kinds run one: keyword expansion for snippets, favicon fetching
        // for quicklinks.
        services: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::extensions::snippets::favicons::{FaviconError, Image};

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
    struct FakeOpener(Mutex<Vec<String>>, Mutex<Vec<Option<String>>>);

    impl FakeOpener {
        fn opened(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }

        fn browsers(&self) -> Vec<Option<String>> {
            self.1.lock().unwrap().clone()
        }
    }

    impl OpenUrl for FakeOpener {
        fn open(&self, url: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(url.to_string());
            Ok(())
        }

        fn open_in(&self, url: &str, browser: Option<&str>) -> Result<(), String> {
            self.1.lock().unwrap().push(browser.map(str::to_string));
            self.open(url)
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

    /// A quicklink extension whose favicon cache is already filled for `hosts`,
    /// with no fetching behind it.
    fn with_favicons(hosts: &[&str], enabled: bool) -> (Fixture, Arc<FaviconCache>) {
        let cache = FaviconCache::in_temp();
        for host in hosts {
            cache.seed(host);
        }
        let mut fixture = fixture(Kind::Quicklink);
        fixture.extension = fixture.extension.with_favicons(
            cache.clone(),
            Arc::new(NoSource),
            Arc::new(move || enabled),
        );
        (fixture, cache)
    }

    struct NoSource;

    impl FaviconSource for NoSource {
        fn fetch(&self, _site: &url::Url) -> Result<Image, FaviconError> {
            unreachable!("the icon is seeded, so nothing is fetched")
        }
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

    fn web_search_rows(items: &[Candidate]) -> Vec<&Candidate> {
        items
            .iter()
            .filter(|c| c.source == Source::WebSearch)
            .collect()
    }

    #[tokio::test]
    async fn a_web_search_is_offered_for_a_typed_query() {
        let (f, cache) = with_favicons(&["duckduckgo.com"], true);
        f.records
            .save(None, "DuckDuckGo", SEARCH, None, None, Some(true))
            .unwrap();

        let items = provider(&f.extension).items("rust traits".into()).await;

        let rows = web_search_rows(&items);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Search DuckDuckGo for “rust traits”");
        assert_eq!(rows[0].icon, cache.path("duckduckgo.com"));
        let actions: Vec<&str> = rows[0].actions.iter().map(|a| a.title.as_str()).collect();
        assert_eq!(actions, ["Open", "Copy"]);
        assert!(
            items
                .iter()
                .any(|c| c.source == Source::RootItem && c.title == "DuckDuckGo"),
            "the quicklink itself is still offered"
        );
    }

    #[tokio::test]
    async fn web_searches_follow_the_file_and_skip_what_cannot_be_one() {
        let f = fixture(Kind::Quicklink);
        f.records
            .reload(&format!(
                "[zeta]\nname = \"Zeta\"\nurl = \"{SEARCH}\"\nfallback = true\n\n\
                 [alpha]\nname = \"Alpha\"\nurl = \"{SEARCH}\"\nfallback = true\n\n\
                 [docs]\nname = \"Docs\"\nurl = \"https://example.com\"\nfallback = true\n\n\
                 [\"a:b\"]\nname = \"Colon\"\nurl = \"{SEARCH}\"\nfallback = true\n\n\
                 [plain]\nname = \"Plain\"\nurl = \"{SEARCH}\"\n"
            ))
            .unwrap();

        let items = provider(&f.extension).items("x".into()).await;
        let titles: Vec<&str> = web_search_rows(&items)
            .iter()
            .map(|c| c.title.as_str())
            .collect();
        assert_eq!(titles, ["Search Zeta for “x”", "Search Alpha for “x”"]);

        let empty = provider(&f.extension).items("  ".into()).await;
        assert!(web_search_rows(&empty).is_empty());
    }

    async fn web_search_id(f: &Fixture, query: &str) -> String {
        let items = provider(&f.extension).items(query.into()).await;
        web_search_rows(&items)[0].id.clone()
    }

    #[tokio::test]
    async fn confirming_a_web_search_opens_it_with_the_query() {
        let f = fixture(Kind::Quicklink);
        f.records
            .save(
                None,
                "Search",
                "https://example.com/s?q={{ query }}&safe=1",
                None,
                Some("firefox"),
                Some(true),
            )
            .unwrap();
        let id = web_search_id(&f, "a&b?c").await;

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(outcome, ActionOutcome::Done);
        let opened = f.opener.opened();
        assert!(opened[0].contains("a%26b%3Fc"), "{}", opened[0]);
        assert!(opened[0].ends_with("&safe=1"), "{}", opened[0]);
        assert_eq!(f.opener.browsers(), vec![Some("firefox".to_string())]);
    }

    #[tokio::test]
    async fn a_web_search_fills_the_clipboard_without_a_form() {
        let f = fixture(Kind::Quicklink);
        f.records
            .save(
                None,
                "Search",
                "https://example.com/s?q={{ query }}&ref={{ clipboard }}",
                None,
                None,
                Some(true),
            )
            .unwrap();
        let id = web_search_id(&f, "dango").await;

        let outcome = f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(outcome, ActionOutcome::Done);
        assert!(f.opener.opened()[0].starts_with("https://example.com/s?q=dango&ref="));
    }

    #[tokio::test]
    async fn copying_a_web_search_copies_its_url() {
        let f = fixture(Kind::Quicklink);
        f.records
            .save(None, "DuckDuckGo", SEARCH, None, None, Some(true))
            .unwrap();
        let id = web_search_id(&f, "rust traits").await;

        let outcome = f.extension.perform_action(&id, ACTION_COPY, &no_values());

        let ActionOutcome::CopyToClipboard(url) = outcome else {
            panic!("expected a copy, got {outcome:?}");
        };
        assert!(url.starts_with("https://duckduckgo.com/?q=rust"), "{url}");
        assert!(f.opener.opened().is_empty());
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
        assert_eq!(candidates[0].status, None, "no body preview in root");
    }

    #[test]
    fn a_multi_line_body_reads_as_one_line() {
        assert_eq!(one_line("Best,\n  Nico"), "Best, Nico");
    }

    #[test]
    fn a_long_body_is_cut_rather_than_wrapped() {
        let subtitle = one_line(&"word ".repeat(40));
        assert_eq!(subtitle.chars().count(), 80);
        assert!(subtitle.ends_with('…'));
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
    fn a_quicklink_without_a_browser_opens_in_the_default() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();

        f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(f.opener.browsers(), vec![None]);
    }

    #[test]
    fn a_quicklink_opens_in_its_browser() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .save(
                None,
                "Docs",
                "https://example.com/docs",
                None,
                Some("firefox"),
                None,
            )
            .unwrap();

        f.extension.perform_action(&id, ACTION_INSERT, &no_values());

        assert_eq!(f.opener.browsers(), vec![Some("firefox".to_string())]);
    }

    #[test]
    fn a_quicklink_with_a_query_opens_in_its_browser() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .save(
                None,
                "Search",
                "https://example.com/s?q={{ query }}",
                None,
                Some("brave"),
                None,
            )
            .unwrap();

        f.extension
            .perform_action(&id, ACTION_INSERT, &values(&[("arg:query", "dango")]));

        assert_eq!(f.opener.opened(), vec!["https://example.com/s?q=dango"]);
        assert_eq!(f.opener.browsers(), vec![Some("brave".to_string())]);
    }

    fn browser_field(tree: &ViewTree) -> Option<FormField> {
        fields(tree)
            .into_iter()
            .find(|field| field.id == FIELD_BROWSER)
    }

    #[test]
    fn the_quicklink_form_offers_the_browsers_of_this_platform() {
        let field = browser_field(&form_tree(None, Kind::Quicklink)).unwrap();

        assert_eq!(field.label, "Browser");
        assert_eq!(field.kind, FieldKind::Dropdown);
        assert_eq!(field.value, None, "the first option, the default, shows");
        let titles: Vec<_> = field.options.iter().map(|o| o.title.as_str()).collect();
        let expected = if cfg!(target_os = "macos") {
            vec![
                "Default browser",
                "Google Chrome",
                "Microsoft Edge",
                "Firefox",
                "Brave",
                "Safari",
            ]
        } else {
            vec![
                "Default browser",
                "Google Chrome",
                "Microsoft Edge",
                "Firefox",
                "Brave",
            ]
        };
        assert_eq!(titles, expected);
        assert_eq!(field.options[0].id, Browser::DEFAULT_ID);
    }

    #[test]
    fn the_edit_form_holds_the_stored_browser() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .save(
                None,
                "Docs",
                "https://example.com/docs",
                None,
                Some("safari"),
                None,
            )
            .unwrap();

        let ActionOutcome::Replaced(tree) =
            f.extension.perform_action(&id, ACTION_EDIT, &no_values())
        else {
            panic!("expected the edit form");
        };

        let field = browser_field(&tree).unwrap();
        assert_eq!(field.value.as_deref(), Some("safari"));
    }

    #[test]
    fn saving_the_form_stores_the_chosen_browser() {
        let f = fixture(Kind::Quicklink);

        f.extension.perform_action(
            "",
            ACTION_SAVE,
            &values(&[
                (FIELD_NAME, "Docs"),
                (FIELD_BODY, "https://example.com/docs"),
                (FIELD_BROWSER, "edge"),
            ]),
        );

        let all = f.records.all().unwrap();
        assert_eq!(all[0].browser.as_deref(), Some("edge"));
    }

    #[test]
    fn the_snippet_form_has_no_browser() {
        assert!(browser_field(&form_tree(None, Kind::Snippet)).is_none());
    }

    fn fallback_field(tree: &ViewTree) -> Option<FormField> {
        fields(tree).into_iter().find(|f| f.id == FIELD_FALLBACK)
    }

    const SEARCH: &str = "https://duckduckgo.com/?q={{ query }}";

    #[test]
    fn the_quicklink_form_offers_web_search_off_by_default() {
        let field = fallback_field(&form_tree(None, Kind::Quicklink)).unwrap();
        assert_eq!(field.label, "Use for web search");
        assert_eq!(field.kind, FieldKind::Toggle);
        assert_eq!(field.value.as_deref(), Some("false"));
        assert!(fallback_field(&form_tree(None, Kind::Snippet)).is_none());
    }

    #[test]
    fn the_edit_form_shows_a_web_search_as_on() {
        let f = fixture(Kind::Quicklink);
        let id = f
            .records
            .save(None, "DuckDuckGo", SEARCH, None, None, Some(true))
            .unwrap();

        let ActionOutcome::Replaced(tree) =
            f.extension.perform_action(&id, ACTION_EDIT, &no_values())
        else {
            panic!("expected the edit form");
        };

        assert_eq!(
            fallback_field(&tree).unwrap().value.as_deref(),
            Some("true")
        );
    }

    #[test]
    fn saving_the_form_turns_web_search_on_and_off() {
        let f = fixture(Kind::Quicklink);
        let save = |id: &str, on: &str| {
            f.extension.perform_action(
                id,
                ACTION_SAVE,
                &values(&[
                    (FIELD_NAME, "DuckDuckGo"),
                    (FIELD_BODY, SEARCH),
                    (FIELD_FALLBACK, on),
                ]),
            )
        };

        save("", "true");
        let id = f.records.all().unwrap()[0].id.clone();
        assert!(f.records.get(&id).unwrap().fallback);

        save(&id, "false");
        assert!(!f.records.get(&id).unwrap().fallback);
    }

    #[test]
    fn a_web_search_must_ask_only_for_the_query() {
        let f = fixture(Kind::Quicklink);
        for url in [
            "https://example.com/docs",
            "https://example.com/s?q={{ query }}&in={{ site }}",
        ] {
            let outcome = f.extension.perform_action(
                "",
                ACTION_SAVE,
                &values(&[
                    (FIELD_NAME, "Search"),
                    (FIELD_BODY, url),
                    (FIELD_FALLBACK, "true"),
                ]),
            );
            assert_eq!(
                outcome,
                ActionOutcome::Failed(
                    "For web search, the URL must ask only for {{ query }}".into()
                ),
                "{url}"
            );
        }
        assert!(f.records.all().unwrap().is_empty());

        f.extension.perform_action(
            "",
            ACTION_SAVE,
            &values(&[
                (FIELD_NAME, "Search"),
                (
                    FIELD_BODY,
                    "https://example.com/s?q={{ query }}&ref={{ clipboard }}",
                ),
                (FIELD_FALLBACK, "true"),
            ]),
        );
        assert!(f.records.all().unwrap()[0].is_web_search());
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

    #[tokio::test]
    async fn a_quicklink_shows_the_favicon_of_the_site_it_opens() {
        let (fixture, cache) = with_favicons(&["example.com"], true);
        fixture
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        let items = provider(&fixture.extension).items(String::new()).await;
        assert_eq!(
            items[0].icon.as_deref(),
            cache.path("example.com").as_deref()
        );
    }

    #[tokio::test]
    async fn two_quicklinks_into_one_site_share_its_favicon() {
        let (fixture, cache) = with_favicons(&["example.com"], true);
        fixture
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        fixture
            .records
            .create("Search", "https://example.com/s?q={{query}}", None)
            .unwrap();
        let items = provider(&fixture.extension).items(String::new()).await;
        let expected = cache.path("example.com");
        assert_eq!(items[0].icon, expected);
        assert_eq!(items[1].icon, expected);
    }

    #[tokio::test]
    async fn a_quicklink_without_a_favicon_keeps_the_link_icon() {
        let (fixture, _) = with_favicons(&["example.com"], true);
        fixture
            .records
            .create("Other", "https://other.example/", None)
            .unwrap();
        let items = provider(&fixture.extension).items(String::new()).await;
        assert_eq!(items[0].icon, Some(format!("{NAMED_ICON}link")));
    }

    #[tokio::test]
    async fn favicons_turned_off_leave_every_quicklink_with_the_link_icon() {
        let (fixture, _) = with_favicons(&["example.com"], false);
        fixture
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        let items = provider(&fixture.extension).items(String::new()).await;
        assert_eq!(items[0].icon, Some(format!("{NAMED_ICON}link")));
    }

    #[test]
    fn the_quicklinks_list_shows_the_same_icons_as_root_search() {
        let (fixture, cache) = with_favicons(&["example.com"], true);
        fixture
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        fixture
            .records
            .create("Other", "https://other.example/", None)
            .unwrap();
        let items = items(&fixture.extension.list_tree());
        assert_eq!(items[0].icon, cache.path("example.com"));
        assert_eq!(items[1].icon, Some(format!("{NAMED_ICON}link")));
    }

    #[test]
    fn a_quicklink_is_never_shown_with_the_snippet_icon() {
        let fixture = fixture(Kind::Quicklink);
        fixture
            .records
            .create("Docs", "https://example.com/docs", None)
            .unwrap();
        let items = items(&fixture.extension.list_tree());
        assert_eq!(items[0].icon, Some(format!("{NAMED_ICON}link")));
    }

    #[test]
    fn a_snippet_list_keeps_the_snippet_icon() {
        let fixture = fixture(Kind::Snippet);
        fixture.records.create("Sig", "Kind regards", None).unwrap();
        let items = items(&fixture.extension.list_tree());
        assert_eq!(items[0].icon, Some(format!("{NAMED_ICON}clipboard-type")));
    }

    fn provider(extension: &SnippetsExtension) -> Arc<dyn crate::search::RootProvider> {
        extension.root_provider().expect("quicklinks provide items")
    }

    #[test]
    fn a_quicklink_extension_without_a_cache_still_works() {
        let fixture = fixture(Kind::Quicklink);
        assert!(
            fixture.extension.services().is_empty(),
            "the favicon service arrives with the cache, not before it"
        );
    }

    #[test]
    fn wiring_favicons_adds_the_service_to_quicklinks_only() {
        let (fixture, _) = with_favicons(&[], true);
        assert_eq!(fixture.extension.services().len(), 1);

        let snippets = fixture_with(Kind::Snippet, FakeTarget::working())
            .extension
            .with_favicons(
                FaviconCache::in_temp(),
                Arc::new(NoSource),
                Arc::new(|| true),
            );
        assert_eq!(
            snippets.services().len(),
            1,
            "a snippet keeps its keyword expansion and gains nothing"
        );
    }

    #[tokio::test]
    async fn five_hundred_quicklinks_all_find_their_cached_icons() {
        let (fixture, _) = with_favicons(&[], true);
        for i in 0..500 {
            fixture
                .records
                .create(
                    &format!("link {i}"),
                    &format!("https://host{i}.example/"),
                    None,
                )
                .unwrap();
        }
        // Every one resolves to a cached icon, which is the slowest case: each
        // record is parsed, rendered and looked up.
        for i in 0..500 {
            fixture
                .extension
                .icons
                .favicons
                .as_ref()
                .unwrap()
                .seed(&format!("host{i}.example"));
        }

        let candidates = provider(&fixture.extension).items(String::new()).await;

        assert_eq!(candidates.len(), 500);
        assert!(
            candidates.iter().all(|candidate| !is_named(candidate)),
            "every one should have found its icon"
        );
    }

    fn is_named(candidate: &Candidate) -> bool {
        candidate
            .icon
            .as_deref()
            .is_some_and(|icon| icon.starts_with(NAMED_ICON))
    }
}
