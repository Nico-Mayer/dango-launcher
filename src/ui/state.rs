use std::collections::HashMap;

use crate::extension::{ActionOutcome, FormValues};
use crate::protocol::{
    Action, ActionStyle, FieldKind, Filter, Filtering, FormField, GridItem, GridView, ListItem,
    Section, View, ViewTree, FILTER_ACTION, GRID_COLUMNS, PROTOCOL_VERSION,
};

use super::effect::{ActionRequest, Effect, Origin};
use super::input::{Chord, Context, Event, Input, Intent, ResultItem, TemplateInspection};
use crate::ranking::fuzzy;

/// All interaction state of the launcher. A pure reducer: input goes in,
/// effects for the shell come out, and `snapshot` says what to draw.
#[derive(Default)]
pub struct Launcher {
    pub(super) activations: u64,
    pub(super) probe: Option<u64>,
    revisions: u64,
    pub(super) query: String,
    pub(super) query_revision: u64,
    pub(super) results: Vec<ResultItem>,
    /// What the keyboard or a click last picked. Results arriving without it
    /// fall back to the first row instead of leaving nothing selected.
    picked: Option<String>,
    pub(super) stack: Vec<PushedView>,
    /// The extension whose command pushed what is on the stack, so an action
    /// chosen inside its view goes back to it.
    owner: Option<String>,
    /// The run whose tree is on top. A streaming command replaces its own view
    /// as the answer grows rather than pushing another.
    shown_invocation: Option<u64>,
    last_invocation: Option<u64>,
    /// Cancelling a streaming command does not recall a tree already in
    /// flight, which would otherwise land as a new view after the user left.
    /// Invocation ids only go up, so one high-water mark covers every run at
    /// or before the one abandoned.
    abandoned_invocation: Option<u64>,
    panel: Option<PanelState>,
    /// The title of a command invoked from root that has neither finished nor
    /// shown a view yet.
    pub(super) working: Option<String>,
    /// A tree that cannot be drawn, shown in place of a view until Escape.
    pub(super) view_error: Option<ViewError>,
    pub(super) toast: Option<HeldToast>,
}

/// A toast the launcher holds until it expires or is cleared. Busy toasts are
/// not held: they follow the work that shows them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HeldToast {
    id: u64,
    pub(super) style: ToastStyle,
    pub(super) text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewError {
    /// A protocol version this build does not know.
    Unsupported,
    /// A tree that breaks the protocol's rules, such as a grid's column count.
    Invalid,
}

/// A selectable item of a list or a grid.
#[derive(Clone, Copy)]
pub(super) enum Entry<'a> {
    List(&'a ListItem),
    Grid(&'a GridItem),
}

impl<'a> Entry<'a> {
    pub(super) fn id(self) -> &'a str {
        match self {
            Self::List(item) => &item.id,
            Self::Grid(item) => &item.id,
        }
    }

    fn actions(self) -> &'a [Action] {
        match self {
            Self::List(item) => &item.actions,
            Self::Grid(item) => &item.actions,
        }
    }
}

/// What the launcher's filter and sections need of an item.
pub(super) trait Grouped {
    fn title(&self) -> &str;
    fn keywords(&self) -> &[String];
    fn section(&self) -> Option<&str>;
}

impl Grouped for ListItem {
    fn title(&self) -> &str {
        &self.title
    }
    fn keywords(&self) -> &[String] {
        &self.keywords
    }
    fn section(&self) -> Option<&str> {
        self.section.as_deref()
    }
}

impl Grouped for GridItem {
    fn title(&self) -> &str {
        &self.title
    }
    fn keywords(&self) -> &[String] {
        &self.keywords
    }
    fn section(&self) -> Option<&str> {
        self.section.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToastStyle {
    Busy,
    Success,
    Failure,
}

pub(super) struct PushedView {
    pub(super) tree: ViewTree,
    /// The command it belongs to, named in the ActionBar.
    pub(super) context: Context,
    pub(super) query: String,
    pub(super) query_revision: u64,
    picked: Option<String>,
    /// The filter option chosen since the last tree, which shows until the
    /// command answers.
    pub(super) filter: Option<String>,
    pub(super) form: FormState,
}

#[derive(Default)]
pub(super) struct FormState {
    pub(super) values: FormValues,
    pub(super) inspections: HashMap<String, TemplateInspection>,
    pub(super) revision: u64,
}

#[derive(Default)]
struct PanelState {
    picked: Option<String>,
    filter: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Error,
    Panel,
    Root,
    List,
    Grid,
    Detail,
    Form,
}

impl Launcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, input: Input) -> Vec<Effect> {
        let mut effects = Vec::new();
        match input {
            Input::Intent(intent) => self.intent(intent, &mut effects),
            Input::Event(event) => self.event(event, &mut effects),
        }
        if self.panel.is_some() && self.available_actions().is_empty() {
            self.panel = None;
        }
        effects
    }

    fn intent(&mut self, intent: Intent, fx: &mut Vec<Effect>) {
        if matches!(
            intent,
            Intent::QueryChanged { .. }
                | Intent::FieldChanged { .. }
                | Intent::Key { .. }
                | Intent::Activate { .. }
                | Intent::TogglePanel
                | Intent::ClosePanel
                | Intent::PanelFilterChanged { .. }
        ) {
            self.clear_failure();
        }
        match intent {
            Intent::Ready => {
                fx.push(Effect::Search(self.query.clone()));
                fx.push(Effect::FinishWarmup);
            }
            Intent::Painted { probe } => fx.push(Effect::ReportPaint(probe)),
            Intent::QueryChanged { text } => self.query_changed(text, fx),
            Intent::FieldChanged { id, value } => self.field_changed(id, value, fx),
            Intent::Key { chord } => self.key(&chord, fx),
            Intent::Activate { id } => self.activate(&id, fx),
            Intent::TogglePanel => {
                if self.panel.is_some() {
                    self.panel = None;
                } else if self.mode() != Mode::Error {
                    self.open_panel();
                }
            }
            Intent::ClosePanel => self.panel = None,
            Intent::Blur => fx.push(Effect::Dismiss),
            Intent::FilterChosen { id } => self.choose_filter(id, fx),
            Intent::PanelFilterChanged { text } => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.filter = text;
                    panel.picked = None;
                }
            }
            Intent::GoBack => match self.mode() {
                Mode::Error => self.view_error = None,
                _ if !self.stack.is_empty() => {
                    self.panel = None;
                    self.escape(fx);
                }
                _ => {}
            },
        }
    }

    fn event(&mut self, event: Event, fx: &mut Vec<Effect>) {
        match event {
            Event::Results { query, items } => {
                // A late answer for an older query was cancelled and must not show.
                if query == self.query {
                    self.results = items;
                }
            }
            Event::Rendered {
                owner,
                context,
                invocation,
                tree,
            } => self.rendered(owner, context, invocation, *tree, fx),
            Event::ActionFinished { request, outcome } => {
                self.action_finished(request, outcome, fx)
            }
            Event::Failed(message) => {
                self.show_failure(message);
                self.working = None;
            }
            Event::TemplateInspected {
                field,
                source,
                inspection,
            } => {
                if let Some(view) = self.stack.last_mut() {
                    if view.form.values.get(&field) == Some(&source) {
                        view.form.inspections.insert(field, inspection);
                    }
                }
            }
            Event::Activated { held, probe } => {
                self.activations += 1;
                self.probe = probe;
                if held {
                    fx.push(Effect::Search(self.query.clone()));
                }
            }
            Event::Reset => self.reset(fx),
            Event::ToastExpired { id } => {
                if self.toast.as_ref().is_some_and(|toast| toast.id == id) {
                    self.toast = None;
                }
            }
        }
    }

    /// A failure deliberately survives this.
    ///
    /// An action that pastes hides the launcher before it does the work, so it
    /// can only report a failure once the window is already gone, and whether
    /// that report arrives before or after this reset is a race. The message is
    /// carried to the next activation instead, and cleared as soon as the user
    /// types, acts, or presses Escape.
    fn reset(&mut self, fx: &mut Vec<Effect>) {
        self.query.clear();
        self.query_revision = self.next_revision();
        self.results.clear();
        self.picked = None;
        self.stack.clear();
        self.owner = None;
        // Deliberately not abandoning: a hide for work resets through here too,
        // on behalf of a command that is still wanted and whose next view is
        // what brings the launcher back.
        self.shown_invocation = None;
        self.panel = None;
        self.view_error = None;
        self.working = None;
        fx.push(Effect::Search(String::new()));
    }

    fn rendered(
        &mut self,
        owner: String,
        context: Context,
        invocation: u64,
        tree: ViewTree,
        fx: &mut Vec<Effect>,
    ) {
        if self
            .abandoned_invocation
            .is_some_and(|abandoned| invocation <= abandoned)
        {
            return;
        }
        self.last_invocation = Some(invocation);
        self.working = None;
        if tree.protocol_version != PROTOCOL_VERSION {
            self.view_error = Some(ViewError::Unsupported);
            return;
        }
        if matches!(&tree.view, View::Grid(grid) if !GRID_COLUMNS.contains(&grid.columns)) {
            self.view_error = Some(ViewError::Invalid);
            return;
        }
        self.owner = Some(owner);
        if self.shown_invocation == Some(invocation) && !self.stack.is_empty() {
            self.replace_top(tree, fx);
        } else {
            self.push(tree, context, fx);
        }
        self.shown_invocation = Some(invocation);
    }

    fn action_finished(
        &mut self,
        request: ActionRequest,
        outcome: ActionOutcome,
        fx: &mut Vec<Effect>,
    ) {
        match (outcome, request.origin) {
            // A plain success already hid the launcher. A start reports through
            // the render channel and pushes its own view.
            (ActionOutcome::Done | ActionOutcome::Started, _) => {}
            (ActionOutcome::CopyToClipboard(text), _) => {
                fx.push(Effect::CopyText(text));
                fx.push(Effect::Dismiss);
            }
            // A root action that produces a view opens it. A snippet needing
            // arguments has to ask before it can do anything.
            (ActionOutcome::Replaced(tree), Origin::Root) => {
                let context = self
                    .results
                    .iter()
                    .find(|item| item.id == request.item_id)
                    .map(|item| Context {
                        title: item.title.clone(),
                        icon: item.icon.clone(),
                    })
                    .unwrap_or_default();
                self.owner = Some(request.extension_id);
                self.stack.clear();
                self.push(*tree, context, fx);
                self.shown_invocation = None;
            }
            (ActionOutcome::Removed(_), Origin::Root) => {
                fx.push(Effect::Search(self.query.clone()))
            }
            (ActionOutcome::Replaced(tree) | ActionOutcome::Removed(tree), Origin::View) => {
                if self.stack.is_empty() {
                    self.push(*tree, Context::default(), fx);
                } else {
                    self.replace_top(*tree, fx);
                }
                self.shown_invocation = None;
            }
            (ActionOutcome::Toast(text), _) => self.show_success(text, fx),
            (ActionOutcome::Failed(message), _) => self.show_failure(message),
        }
    }

    fn show_failure(&mut self, text: String) {
        let id = self.next_revision();
        self.toast = Some(HeldToast {
            id,
            style: ToastStyle::Failure,
            text,
        });
    }

    /// A failure stays until the next key press or click.
    fn clear_failure(&mut self) {
        if self
            .toast
            .as_ref()
            .is_some_and(|toast| toast.style == ToastStyle::Failure)
        {
            self.toast = None;
        }
    }

    fn show_success(&mut self, text: String, fx: &mut Vec<Effect>) {
        let id = self.next_revision();
        self.toast = Some(HeldToast {
            id,
            style: ToastStyle::Success,
            text,
        });
        fx.push(Effect::ExpireToast { id });
    }

    fn push(&mut self, tree: ViewTree, context: Context, fx: &mut Vec<Effect>) {
        self.panel = None;
        let query_revision = self.next_revision();
        let mut view = PushedView {
            tree,
            context,
            query: String::new(),
            query_revision,
            picked: None,
            filter: None,
            form: FormState::default(),
        };
        self.seed_form(&mut view, fx);
        self.stack.push(view);
    }

    fn replace_top(&mut self, tree: ViewTree, fx: &mut Vec<Effect>) {
        let Some(mut view) = self.stack.pop() else {
            return;
        };
        view.tree = tree;
        view.filter = None;
        self.seed_form(&mut view, fx);
        self.stack.push(view);
    }

    /// A form takes its values from each tree it is given, so a field the user
    /// never touches still submits what it was showing.
    fn seed_form(&mut self, view: &mut PushedView, fx: &mut Vec<Effect>) {
        let View::Form(form) = &view.tree.view else {
            return;
        };
        view.form = FormState {
            values: form
                .fields
                .iter()
                .filter(|field| field.kind != FieldKind::Separator)
                .map(|field| (field.id.clone(), seeded_value(field)))
                .collect(),
            inspections: HashMap::new(),
            revision: self.next_revision(),
        };
        for field in form.fields.iter().filter(|f| f.kind == FieldKind::Template) {
            fx.push(Effect::InspectTemplate {
                field: field.id.clone(),
                source: field.value.clone().unwrap_or_default(),
            });
        }
    }

    fn next_revision(&mut self) -> u64 {
        self.revisions += 1;
        self.revisions
    }

    fn query_changed(&mut self, text: String, fx: &mut Vec<Effect>) {
        match self.stack.last_mut() {
            None => {
                self.query = text.clone();
                self.picked = None;
                self.clear_failure();
                fx.push(Effect::Search(text));
            }
            Some(view) => {
                view.query = text;
                view.picked = None;
            }
        }
    }

    fn field_changed(&mut self, id: String, value: String, fx: &mut Vec<Effect>) {
        let Some(view) = self.stack.last_mut() else {
            return;
        };
        let View::Form(form) = &view.tree.view else {
            return;
        };
        let Some(field) = form.fields.iter().find(|field| field.id == id) else {
            return;
        };
        if field.kind == FieldKind::Template {
            fx.push(Effect::InspectTemplate {
                field: id.clone(),
                source: value.clone(),
            });
        }
        view.form.values.insert(id, value);
    }

    fn key(&mut self, chord: &Chord, fx: &mut Vec<Effect>) {
        let escape = *chord == Chord::plain("Escape");
        let enter = *chord == Chord::plain("Enter");
        let up = *chord == Chord::plain("ArrowUp");
        let down = *chord == Chord::plain("ArrowDown");
        let left = *chord == Chord::plain("ArrowLeft");
        let right = *chord == Chord::plain("ArrowRight");
        let mode = self.mode();
        match mode {
            Mode::Error => {
                if escape {
                    self.view_error = None;
                }
                return;
            }
            Mode::Panel => {
                if escape {
                    self.panel = None;
                } else if up || down {
                    let actions = self.panel_actions();
                    let next = step(
                        actions.iter().map(|a| a.id.as_str()),
                        self.panel_selected(),
                        down,
                    );
                    if let Some(panel) = self.panel.as_mut() {
                        panel.picked = next;
                    }
                } else if enter {
                    if let Some(id) = self.panel_selected() {
                        self.run_available(&id, fx);
                    }
                }
                return;
            }
            Mode::Root | Mode::List | Mode::Grid | Mode::Detail | Mode::Form => {}
        }

        if escape {
            self.escape(fx);
        } else if (up || down) && mode == Mode::Root {
            let next = step(
                self.root_order().map(|item| item.id.as_str()),
                self.root_selected().map(|item| item.id.clone()),
                down,
            );
            self.picked = next;
        } else if (up || down) && mode == Mode::List {
            let next = step(
                self.entries().into_iter().map(Entry::id),
                self.entry_selected().map(|entry| entry.id().to_string()),
                down,
            );
            if let Some(view) = self.stack.last_mut() {
                view.picked = next;
            }
        } else if mode == Mode::Grid && (up || down || left || right) {
            let next = self.grid_step(chord);
            if let Some(view) = self.stack.last_mut() {
                view.picked = next;
            }
        } else if enter || (mode == Mode::Form && *chord == Chord::command("Enter")) {
            self.confirm(fx);
        } else if *chord == Chord::command("k") {
            self.open_panel();
        } else if let Some(action) = self.available_actions().iter().find(|action| {
            action
                .shortcut
                .as_ref()
                .is_some_and(|s| Chord::from(s) == *chord)
        }) {
            let id = action.id.clone();
            self.run_available(&id, fx);
        }
    }

    fn escape(&mut self, fx: &mut Vec<Effect>) {
        self.clear_failure();
        if !self.stack.is_empty() {
            // A command may still be working behind this view, and the user has
            // left it: stop it rather than paying for an answer nobody reads.
            fx.push(Effect::CancelInvocation);
            self.abandoned_invocation = self.last_invocation;
            self.shown_invocation = None;
            self.stack.pop();
            let revision = self.next_revision();
            match self.stack.last_mut() {
                Some(view) => view.query_revision = revision,
                None => {
                    self.owner = None;
                    self.query_revision = revision;
                }
            }
        } else if !self.query.is_empty() {
            self.query.clear();
            self.query_revision = self.next_revision();
            fx.push(Effect::Search(String::new()));
        } else {
            fx.push(Effect::Dismiss);
        }
    }

    /// Enter on what is showing. A root result that is a command is invoked,
    /// anything else runs its primary action. A command carries no actions,
    /// which is what tells the two apart.
    fn confirm(&mut self, fx: &mut Vec<Effect>) {
        match self.mode() {
            Mode::Root => {
                let Some(item) = self.root_selected().cloned() else {
                    return;
                };
                match item.actions.first() {
                    Some(action) => self.run_root_action(&item, &action.id, fx),
                    None => {
                        self.clear_failure();
                        self.panel = None;
                        self.working = Some(item.title.clone());
                        fx.push(Effect::InvokeCommand {
                            command_id: item.id,
                            query: self.query.clone(),
                        });
                    }
                }
            }
            Mode::List | Mode::Grid => match self.entry_selected() {
                Some(entry) => {
                    if let Some(action) = entry.actions().first() {
                        let (action, item) = (action.id.clone(), entry.id().to_string());
                        self.run_view_action(action, Some(item), None, fx);
                    }
                }
                None => {
                    if let Some(action) = self.empty_actions().first() {
                        let action = action.id.clone();
                        self.run_view_action(action, None, None, fx);
                    }
                }
            },
            Mode::Detail => {
                if let Some(action) = self.view_actions().first() {
                    let action = action.id.clone();
                    self.run_view_action(action, None, None, fx);
                }
            }
            Mode::Form => {
                let Some(view) = self.stack.last() else {
                    return;
                };
                let View::Form(form) = &view.tree.view else {
                    return;
                };
                if let Some(action) = form.actions.first() {
                    let (action, item, values) = (
                        action.id.clone(),
                        form.item_id.clone(),
                        view.form.values.clone(),
                    );
                    self.run_view_action(action, item, Some(values), fx);
                }
            }
            Mode::Error | Mode::Panel => {}
        }
    }

    fn activate(&mut self, id: &str, fx: &mut Vec<Effect>) {
        match self.mode() {
            Mode::Panel => {
                if self
                    .available_actions()
                    .iter()
                    .any(|action| action.id == id)
                {
                    self.run_available(id, fx);
                }
            }
            Mode::Root => {
                if self.results.iter().any(|item| item.id == id) {
                    self.picked = Some(id.to_string());
                    self.confirm(fx);
                }
            }
            Mode::List | Mode::Grid => {
                if self.entries().iter().any(|entry| entry.id() == id) {
                    if let Some(view) = self.stack.last_mut() {
                        view.picked = Some(id.to_string());
                    }
                    self.confirm(fx);
                }
            }
            Mode::Error | Mode::Detail | Mode::Form => {}
        }
    }

    fn open_panel(&mut self) {
        if !self.available_actions().is_empty() {
            self.panel = Some(PanelState::default());
        }
    }

    /// Delivers a filter option to the view's command, keeping the query.
    fn choose_filter(&mut self, id: String, fx: &mut Vec<Effect>) {
        let known = self
            .filter()
            .is_some_and(|filter| filter.options.iter().any(|option| option.id == id));
        if !known {
            return;
        }
        if let Some(view) = self.stack.last_mut() {
            view.filter = Some(id.clone());
        }
        self.run_view_action(FILTER_ACTION.to_string(), Some(id), None, fx);
    }

    /// Runs one of the actions the panel offers, on whatever it applies to.
    fn run_available(&mut self, action: &str, fx: &mut Vec<Effect>) {
        if let Some(option) = action.strip_prefix(FILTER_PREFIX) {
            self.panel = None;
            self.choose_filter(option.to_string(), fx);
            return;
        }
        if self.stack.is_empty() {
            if let Some(item) = self.root_selected().cloned() {
                self.run_root_action(&item, action, fx);
            }
            return;
        }
        let item = self.view_item_id();
        self.run_view_action(action.to_string(), item, None, fx);
    }

    fn run_root_action(&mut self, item: &ResultItem, action: &str, fx: &mut Vec<Effect>) {
        self.clear_failure();
        self.panel = None;
        fx.push(Effect::RunAction(ActionRequest {
            origin: Origin::Root,
            extension_id: item.extension_id.clone(),
            item_id: item.id.clone(),
            action_id: action.to_string(),
            values: None,
            query: Some(self.query.clone()),
        }));
    }

    /// An action chosen inside a pushed view goes to the extension that owns
    /// the running command, through the same path a root result uses.
    fn run_view_action(
        &mut self,
        action: String,
        item: Option<String>,
        values: Option<FormValues>,
        fx: &mut Vec<Effect>,
    ) {
        let Some(owner) = self.owner.clone() else {
            return;
        };
        self.clear_failure();
        self.panel = None;
        fx.push(Effect::RunAction(ActionRequest {
            origin: Origin::View,
            extension_id: owner,
            item_id: item.unwrap_or_default(),
            action_id: action,
            values,
            query: None,
        }));
    }

    pub(super) fn mode(&self) -> Mode {
        if self.view_error.is_some() {
            Mode::Error
        } else if self.panel.is_some() {
            Mode::Panel
        } else {
            self.surface()
        }
    }

    /// What is showing under the action panel.
    pub(super) fn surface(&self) -> Mode {
        match self.stack.last().map(|view| &view.tree.view) {
            None => Mode::Root,
            Some(View::List(_)) => Mode::List,
            Some(View::Grid(_)) => Mode::Grid,
            Some(View::Detail(_)) => Mode::Detail,
            Some(View::Form(_)) => Mode::Form,
        }
    }

    /// Root results in the order they are drawn: pinned, then suggestions,
    /// then everything else.
    pub(super) fn root_order(&self) -> impl Iterator<Item = &ResultItem> {
        let pinned = self.results.iter().filter(|item| item.pinned);
        let suggested = self
            .results
            .iter()
            .filter(|item| !item.pinned && item.suggested);
        let rest = self
            .results
            .iter()
            .filter(|item| !item.pinned && !item.suggested);
        pinned.chain(suggested).chain(rest)
    }

    pub(super) fn root_selected(&self) -> Option<&ResultItem> {
        self.picked
            .as_deref()
            .and_then(|picked| self.results.iter().find(|item| item.id == picked))
            .or_else(|| self.root_order().next())
    }

    /// A pushed list's items after launcher-side narrowing, grouped in section
    /// order, without the groups left empty. Items naming no declared section
    /// come first. A view that owns its own filtering would get each query
    /// change instead, which no command needs yet.
    pub(super) fn view_sections(&self) -> Vec<(Option<&Section>, Vec<&ListItem>)> {
        let Some(view) = self.stack.last() else {
            return Vec::new();
        };
        let View::List(list) = &view.tree.view else {
            return Vec::new();
        };
        group(&list.items, &list.sections, &list.filtering, &view.query)
    }

    /// A pushed grid's tiles, grouped and narrowed as a list's items are.
    pub(super) fn grid_sections(&self) -> Vec<(Option<&Section>, Vec<&GridItem>)> {
        let Some(view) = self.stack.last() else {
            return Vec::new();
        };
        let View::Grid(grid) = &view.tree.view else {
            return Vec::new();
        };
        group(&grid.items, &grid.sections, &grid.filtering, &view.query)
    }

    pub(super) fn grid(&self) -> Option<&GridView> {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::Grid(grid)) => Some(grid),
            _ => None,
        }
    }

    /// The list's items or the grid's tiles in the order they are drawn.
    pub(super) fn entries(&self) -> Vec<Entry<'_>> {
        let list = self
            .view_sections()
            .into_iter()
            .flat_map(|(_, items)| items);
        let grid = self
            .grid_sections()
            .into_iter()
            .flat_map(|(_, items)| items);
        list.map(Entry::List).chain(grid.map(Entry::Grid)).collect()
    }

    pub(super) fn entry_selected(&self) -> Option<Entry<'_>> {
        let entries = self.entries();
        let picked = self.stack.last().and_then(|view| view.picked.as_deref());
        picked
            .and_then(|picked| entries.iter().find(|entry| entry.id() == picked))
            .or_else(|| entries.first())
            .copied()
    }

    pub(super) fn view_selected(&self) -> Option<&ListItem> {
        match self.entry_selected() {
            Some(Entry::List(item)) => Some(item),
            _ => None,
        }
    }

    /// The tile an arrow key moves to. Rows restart under each section, so
    /// Up and Down look for the same column in the row above or below, and
    /// take that row's last tile when it is shorter.
    fn grid_step(&self, chord: &Chord) -> Option<String> {
        let columns = usize::from(self.grid()?.columns);
        let rows: Vec<Vec<&str>> = self
            .grid_sections()
            .into_iter()
            .flat_map(|(_, items)| {
                items
                    .chunks(columns)
                    .map(|row| row.iter().map(|item| item.id.as_str()).collect())
                    .collect::<Vec<_>>()
            })
            .collect();
        let current = self.entry_selected()?.id();
        let (row, column) = rows
            .iter()
            .enumerate()
            .find_map(|(r, ids)| ids.iter().position(|id| *id == current).map(|c| (r, c)))?;
        let flat: Vec<&str> = rows.iter().flatten().copied().collect();
        let index = flat.iter().position(|id| *id == current)?;
        let target = match chord.key.as_str() {
            "arrowleft" => flat[index.saturating_sub(1)],
            "arrowright" => flat[(index + 1).min(flat.len() - 1)],
            "arrowup" if row > 0 => {
                let above = &rows[row - 1];
                above[column.min(above.len() - 1)]
            }
            "arrowdown" if row + 1 < rows.len() => {
                let below = &rows[row + 1];
                below[column.min(below.len() - 1)]
            }
            _ => current,
        };
        Some(target.to_string())
    }

    fn view_actions(&self) -> &[Action] {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::Detail(detail)) => &detail.actions,
            Some(View::Form(form)) => &form.actions,
            Some(View::List(_) | View::Grid(_)) | None => &[],
        }
    }

    /// What the action panel offers. A list item's own actions win over the
    /// view's, because the item is what the user has selected.
    pub(super) fn available_actions(&self) -> &[Action] {
        match self.surface() {
            Mode::Root => self.root_selected().map_or(&[], |item| &item.actions),
            Mode::List | Mode::Grid => match self.entry_selected() {
                Some(entry) => entry.actions(),
                None => self.empty_actions(),
            },
            _ => self.view_actions(),
        }
    }

    /// What an empty list offers. Nothing while it is still loading.
    fn empty_actions(&self) -> &[Action] {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::List(list)) if !list.loading => list
                .empty_state
                .as_ref()
                .map_or(&[], |state| &state.actions),
            Some(View::Grid(grid)) if !grid.loading => grid
                .empty_state
                .as_ref()
                .map_or(&[], |state| &state.actions),
            _ => &[],
        }
    }

    /// The actions in the order the panel lists them: grouped by section in
    /// the order sections first appear, then the view's filter options, with
    /// danger actions after all others, narrowed by the panel's filter.
    pub(super) fn panel_actions(&self) -> Vec<Action> {
        let ordered = self.ordered_actions();
        let (normal, danger): (Vec<&Action>, Vec<&Action>) = ordered
            .into_iter()
            .partition(|action| action.style != ActionStyle::Danger);
        let options = self.filter().into_iter().flat_map(|filter| {
            filter.options.iter().map(|option| Action {
                id: format!("{FILTER_PREFIX}{}", option.id),
                title: option.title.clone(),
                section: Some("Filter".to_string()),
                ..Default::default()
            })
        });
        let mut actions: Vec<Action> = normal
            .into_iter()
            .cloned()
            .chain(options)
            .chain(danger.into_iter().cloned())
            .collect();
        if let Some(filter) = self.panel.as_ref().map(|panel| &panel.filter) {
            if !filter.is_empty() {
                let mut query = fuzzy::Query::new(filter);
                actions.retain(|action| query.score(&action.title).is_some());
            }
        }
        actions
    }

    /// The view's filter dropdown, if it declares one.
    pub(super) fn filter(&self) -> Option<&Filter> {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::List(list)) => list.filter.as_ref(),
            Some(View::Grid(grid)) => grid.filter.as_ref(),
            _ => None,
        }
    }

    /// The option the dropdown shows: one just chosen, else the tree's.
    pub(super) fn filter_selected(&self) -> Option<String> {
        let filter = self.filter()?;
        let chosen = self.stack.last().and_then(|view| view.filter.clone());
        Some(chosen.unwrap_or_else(|| filter.selected.clone()))
    }

    pub(super) fn panel_filter(&self) -> &str {
        self.panel
            .as_ref()
            .map_or("", |panel| panel.filter.as_str())
    }

    fn ordered_actions(&self) -> Vec<&Action> {
        let actions = self.available_actions();
        let mut sections: Vec<Option<&str>> = Vec::new();
        for action in actions {
            if !sections.contains(&action.section.as_deref()) {
                sections.push(action.section.as_deref());
            }
        }
        let pass = |danger: bool| {
            sections.iter().flat_map(move |section| {
                actions.iter().filter(move |action| {
                    action.section.as_deref() == *section
                        && (action.style == ActionStyle::Danger) == danger
                })
            })
        };
        pass(false).chain(pass(true)).collect()
    }

    pub(super) fn panel_selected(&self) -> Option<String> {
        let panel = self.panel.as_ref()?;
        let actions = self.panel_actions();
        panel
            .picked
            .as_deref()
            .filter(|picked| actions.iter().any(|action| action.id == *picked))
            .or_else(|| actions.first().map(|action| action.id.as_str()))
            .map(str::to_string)
    }

    pub(super) fn panel_open(&self) -> bool {
        self.panel.is_some()
    }

    /// Which item an action applies to. Lists use their selection, forms carry
    /// the record that opened them, and detail views have none.
    fn view_item_id(&self) -> Option<String> {
        match self.stack.last().map(|view| &view.tree.view) {
            Some(View::List(_) | View::Grid(_)) => {
                self.entry_selected().map(|entry| entry.id().to_string())
            }
            Some(View::Form(form)) => form.item_id.clone(),
            Some(View::Detail(_)) | None => None,
        }
    }
}

/// Marks a panel entry as a filter option rather than an action.
pub(super) const FILTER_PREFIX: &str = "dango.filter:";

/// Items in section order, narrowed by the query when the launcher filters,
/// without the groups left empty. Items naming no declared section come first.
fn group<'a, T: Grouped>(
    items: &'a [T],
    sections: &'a [Section],
    filtering: &Filtering,
    query: &str,
) -> Vec<(Option<&'a Section>, Vec<&'a T>)> {
    let needle = query.to_lowercase();
    let shown = |item: &&T| {
        *filtering != Filtering::Launcher
            || needle.is_empty()
            || item.title().to_lowercase().contains(&needle)
            || item
                .keywords()
                .iter()
                .any(|keyword| keyword.to_lowercase().contains(&needle))
    };
    let declared = |id: &str| sections.iter().any(|section| section.id == id);
    let loose = items
        .iter()
        .filter(|item| item.section().is_none_or(|id| !declared(id)))
        .filter(shown)
        .collect();
    let mut groups = vec![(None, loose)];
    for section in sections {
        let members = items
            .iter()
            .filter(|item| item.section() == Some(section.id.as_str()))
            .filter(shown)
            .collect();
        groups.push((Some(section), members));
    }
    groups.retain(|(_, members): &(_, Vec<_>)| !members.is_empty());
    groups
}

/// What a field submits until the user changes it. A dropdown without a value
/// holds its first option, as that is what it shows.
pub(super) fn seeded_value(field: &FormField) -> String {
    match (&field.value, field.kind == FieldKind::Dropdown) {
        (Some(value), _) => value.clone(),
        (None, true) => field
            .options
            .first()
            .map(|option| option.id.clone())
            .unwrap_or_default(),
        (None, false) => String::new(),
    }
}

/// The id one row up or down from `current`, stopping at either end.
fn step<'a>(
    ids: impl Iterator<Item = &'a str>,
    current: Option<String>,
    down: bool,
) -> Option<String> {
    let ids: Vec<&str> = ids.collect();
    let index = current
        .as_deref()
        .and_then(|current| ids.iter().position(|id| *id == current));
    let next = match (index, down) {
        (None, _) => 0,
        (Some(i), true) => (i + 1).min(ids.len().saturating_sub(1)),
        (Some(i), false) => i.saturating_sub(1),
    };
    ids.get(next).map(|id| id.to_string())
}
