use std::collections::HashMap;

use crate::extension::{ActionOutcome, FormValues};
use crate::protocol::{
    Action, ActionStyle, FieldKind, Filter, FormField, GridItem, GridView, ListItem, Modifier,
    Section, Shortcut, View, ViewTree, FILTER_ACTION, GRID_COLUMNS, PROTOCOL_VERSION,
};

use super::effect::{ActionRequest, BusyStage, Effect, Origin};
use super::input::{Chord, Context, Event, Input, Intent, ResultItem, TemplateInspection};
use crate::ranking::fuzzy;
use crate::tasks::{TaskState, TaskStatus};

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
    /// Whether `results` is the last answer for `query`. Until it is, more
    /// providers may still add rows.
    pub(super) results_complete: bool,
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
    /// The empty-query list was opened with Arrow Down. Without it an empty
    /// query shows only pinned items.
    root_open: bool,
    /// The typed query has shown the card, rows or "No results". From then on
    /// the card stays while later keystrokes search, so it does not drop to
    /// the pill and back on every key.
    pub(super) typed_card: bool,
    /// The title of a command invoked from root that has neither finished nor
    /// shown a view yet.
    pub(super) working: Option<String>,
    /// A tree that cannot be drawn, shown in place of a view until Escape.
    pub(super) view_error: Option<ViewError>,
    pub(super) ticker: Option<HeldTicker>,
    pub(super) busy_marks: BusyMarks,
    /// The last report of every task that has not ended.
    pub(super) tasks: Vec<TaskState>,
    /// A task's result that came while the launcher was hidden.
    pending_result: Option<TaskState>,
    /// What the launcher reports while it is off screen. Set while it is still
    /// visible, it waits for the hide.
    pub(super) notice: Option<HeldTicker>,
    pub(super) visible: bool,
}

/// A ticker the launcher holds until it expires or is cleared. Busy tickers are
/// not held: they follow the work that shows them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HeldTicker {
    pub(super) id: u64,
    pub(super) style: TickerStyle,
    pub(super) text: String,
    pub(super) action: Option<TickerAction>,
}

/// One action a ticker offers, run by its chord while the ticker shows.
#[derive(Clone, Debug, PartialEq)]
pub struct TickerAction {
    pub title: String,
    pub shortcut: Shortcut,
    pub request: ActionRequest,
}

/// The loading bar and busy ticker wait a moment before they appear and, once
/// shown, stay a moment, so fast work never blinks them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum BusyMarks {
    #[default]
    Hidden,
    Pending {
        id: u64,
    },
    Shown {
        id: u64,
        held: bool,
        text: String,
        /// Shown for a background task, which draws no loading bar. Kept while
        /// the marks linger after the work ends.
        task: bool,
    },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickerStyle {
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

    /// The extension whose views are on screen, if any.
    pub fn view_owner(&self) -> Option<&str> {
        if self.stack.is_empty() {
            return None;
        }
        self.owner.as_deref()
    }

    pub fn apply(&mut self, input: Input) -> Vec<Effect> {
        let mut effects = Vec::new();
        match input {
            Input::Intent(intent) => self.intent(intent, &mut effects),
            Input::Event(event) => self.event(event, &mut effects),
        }
        self.settle_busy_marks(&mut effects);
        if self.panel.is_some() && self.available_actions().is_empty() {
            self.panel = None;
        }
        effects
    }

    fn intent(&mut self, intent: Intent, fx: &mut Vec<Effect>) {
        if let Intent::Key { chord } = &intent {
            if let Some(action) = self
                .ticker_action()
                .filter(|action| Chord::from(&action.shortcut) == *chord)
            {
                fx.push(Effect::RunAction(action.request.clone()));
                self.ticker = None;
                return;
            }
        }
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
            self.clear_lasting();
        }
        match intent {
            Intent::Ready => fx.push(Effect::Search(self.query.clone())),
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
            Intent::ClickedOutside => fx.push(Effect::HideAndHold),
        }
    }

    fn event(&mut self, event: Event, fx: &mut Vec<Effect>) {
        match event {
            Event::Results {
                query,
                items,
                complete,
            } => {
                // A late answer for an older query was cancelled and must not show.
                if query == self.query {
                    if !query.is_empty() && (complete || !items.is_empty()) {
                        self.typed_card = true;
                    }
                    self.results = items;
                    self.results_complete = complete;
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
                self.show_failure(message, fx);
                self.working = None;
                self.busy_marks = BusyMarks::Hidden;
            }
            Event::HotkeyFailed(message) => self.show_failure(message, fx),
            Event::LateSuccess(message) => {
                self.show_notice(TickerStyle::Success, message, None, fx)
            }
            Event::Succeeded(message) => {
                self.show_notice(TickerStyle::Success, message, None, fx);
                self.working = None;
                self.busy_marks = BusyMarks::Hidden;
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
            Event::Activated { held, probe, asked } => {
                self.visible = true;
                let jump = self.notice.take().and_then(|notice| notice.action);
                self.activations += 1;
                self.probe = probe;
                if held {
                    fx.push(Effect::Search(self.query.clone()));
                }
                if let Some(result) = self.pending_result.take() {
                    let in_its_view = (held && self.shows_view_of(&result))
                        || self.shows_progress_of(&result.owner);
                    match jump.filter(|_| asked) {
                        _ if in_its_view => {}
                        Some(jump) => fx.push(Effect::RunAction(jump.request)),
                        None => self.show_task_result(result, fx),
                    }
                }
            }
            Event::Reset => self.reset(fx),
            Event::Hidden => {
                self.visible = false;
                // A command that pastes asks for the hide and goes on working,
                // so its failure can land just before the window is gone.
                if let Some(ticker) = self
                    .ticker
                    .take_if(|ticker| ticker.style == TickerStyle::Failure)
                {
                    self.show_notice(TickerStyle::Failure, ticker.text, None, fx);
                    return;
                }
                if let Some(notice) = &self.notice {
                    fx.push(Effect::ExpireTicker {
                        id: notice.id,
                        style: notice.style,
                    });
                }
            }
            Event::BusyMarksWoke { id } => self.busy_marks_woke(id, fx),
            Event::Task(state) => self.task_changed(*state, fx),
            Event::TickerExpired { id } => {
                if self.ticker.as_ref().is_some_and(|ticker| ticker.id == id) {
                    self.ticker = None;
                }
                if self.notice.as_ref().is_some_and(|notice| notice.id == id) {
                    self.notice = None;
                }
            }
        }
    }

    fn reset(&mut self, fx: &mut Vec<Effect>) {
        self.query.clear();
        self.query_revision = self.next_revision();
        self.results.clear();
        self.results_complete = false;
        self.picked = None;
        self.root_open = false;
        self.typed_card = false;
        self.stack.clear();
        self.owner = None;
        // Deliberately not abandoning: a hide for work resets through here too,
        // on behalf of a command that is still wanted and whose next view is
        // what brings the launcher back.
        self.shown_invocation = None;
        self.panel = None;
        self.view_error = None;
        self.working = None;
        self.ticker = None;
        self.busy_marks = BusyMarks::Hidden;
        fx.push(Effect::Search(String::new()));
    }

    /// What the busy ticker says while Dango is busy, and nothing otherwise.
    pub(super) fn busy_text(&self) -> Option<String> {
        if let Some(title) = &self.working {
            return Some(format!("Running {title}…"));
        }
        if self.top_loading() {
            return Some("Loading…".to_string());
        }
        self.busy_task().map(|task| task.line.clone())
    }

    pub(super) fn top_loading(&self) -> bool {
        self.stack
            .last()
            .is_some_and(|view| is_loading(&view.tree.view))
    }

    /// The oldest running task the user can't see in its own view.
    fn busy_task(&self) -> Option<&TaskState> {
        self.tasks
            .iter()
            .find(|task| task.status == TaskStatus::Running && !self.shows_progress_of(&task.owner))
    }

    /// The view the task itself streamed into, which a held launcher comes
    /// back to with the result already in it.
    fn shows_view_of(&self, task: &TaskState) -> bool {
        self.owner.as_deref() == Some(task.owner.as_str())
            && self
                .stack
                .last()
                .is_some_and(|view| view.context.title == task.title)
    }

    /// Whether the view on top is a progress view the task's owner pushed.
    fn shows_progress_of(&self, owner: &str) -> bool {
        matches!(
            self.stack.last().map(|view| &view.tree.view),
            Some(View::Progress(_))
        ) && self.owner.as_deref() == Some(owner)
    }

    fn task_changed(&mut self, state: TaskState, fx: &mut Vec<Effect>) {
        let known = self.tasks.iter().position(|task| task.id == state.id);
        let was = known.map(|index| self.tasks[index].status.clone());
        match known {
            Some(index) if state.status.ended() => {
                self.tasks.remove(index);
            }
            Some(index) => self.tasks[index] = state.clone(),
            None if !state.status.ended() => self.tasks.push(state.clone()),
            None => {}
        }
        let arrived = match &state.status {
            TaskStatus::NeedsInput { .. } => !matches!(was, Some(TaskStatus::NeedsInput { .. })),
            TaskStatus::Done { .. } | TaskStatus::Failed { .. } => true,
            TaskStatus::Running | TaskStatus::Cancelled => false,
        };
        if !arrived {
            return;
        }
        if !self.visible {
            match &state.status {
                TaskStatus::Failed { sentence } => {
                    self.show_notice(TickerStyle::Failure, sentence.clone(), None, fx)
                }
                TaskStatus::Done { sentence } | TaskStatus::NeedsInput { sentence } => {
                    let action = show_answer(&state);
                    self.show_notice(TickerStyle::Success, sentence.clone(), action, fx)
                }
                TaskStatus::Running | TaskStatus::Cancelled => {}
            }
            self.pending_result = Some(state);
        } else if !self.shows_progress_of(&state.owner) {
            self.show_task_result(state, fx);
        }
    }

    fn show_task_result(&mut self, state: TaskState, fx: &mut Vec<Effect>) {
        match &state.status {
            TaskStatus::NeedsInput { sentence } | TaskStatus::Done { sentence } => {
                let action = show_answer(&state);
                self.show_success(sentence.clone(), action, fx);
            }
            TaskStatus::Failed { sentence } => {
                let id = self.next_revision();
                self.ticker = Some(HeldTicker {
                    id,
                    style: TickerStyle::Failure,
                    text: sentence.clone(),
                    action: state
                        .retry
                        .as_deref()
                        .map(|retry| task_action(&state, "Retry", "r", retry)),
                });
            }
            TaskStatus::Running | TaskStatus::Cancelled => {}
        }
    }

    /// Whether the busy marks are there for a task rather than for a command
    /// or a loading view, which also draw the SearchBar's bar.
    pub(super) fn busy_shows_task(&self) -> bool {
        matches!(self.busy_marks, BusyMarks::Shown { task: true, .. })
    }

    /// Whether a background task is what makes Dango busy right now.
    fn busy_source_is_task(&self) -> bool {
        self.working.is_none() && !self.top_loading() && self.busy_task().is_some()
    }

    /// The running task's Cancel, while the busy ticker is showing that task.
    pub(super) fn busy_cancel(&self) -> Option<TickerAction> {
        if !self.busy_shows_task() {
            return None;
        }
        let task = self.busy_task()?;
        let cancel = task.cancel.as_deref()?;
        Some(task_action(task, "Cancel", ".", cancel))
    }

    fn settle_busy_marks(&mut self, fx: &mut Vec<Effect>) {
        let busy = self.busy_text();
        let for_task = self.busy_source_is_task();
        match (&mut self.busy_marks, busy) {
            (BusyMarks::Hidden, Some(_)) => {
                let id = self.next_revision();
                self.busy_marks = BusyMarks::Pending { id };
                fx.push(Effect::WakeBusyMarks {
                    id,
                    stage: BusyStage::Delay,
                });
            }
            (BusyMarks::Pending { .. }, None) | (BusyMarks::Shown { held: true, .. }, None) => {
                self.busy_marks = BusyMarks::Hidden;
            }
            (BusyMarks::Shown { text, task, .. }, Some(busy)) => {
                *text = busy;
                *task = for_task;
            }
            _ => {}
        }
    }

    fn busy_marks_woke(&mut self, woke: u64, fx: &mut Vec<Effect>) {
        match &mut self.busy_marks {
            BusyMarks::Pending { id } if *id == woke => {
                self.busy_marks = BusyMarks::Shown {
                    id: woke,
                    held: false,
                    text: self.busy_text().unwrap_or_default(),
                    task: self.busy_source_is_task(),
                };
                fx.push(Effect::WakeBusyMarks {
                    id: woke,
                    stage: BusyStage::Hold,
                });
            }
            BusyMarks::Shown { id, held, .. } if *id == woke && !*held => *held = true,
            _ => {}
        }
    }

    pub(super) fn busy_marks_shown(&self) -> Option<&str> {
        match &self.busy_marks {
            BusyMarks::Shown { text, .. } => Some(text),
            _ => None,
        }
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
        if self.refused(&tree) {
            return;
        }
        // The stack has one owner, who gets its actions: a view from another
        // extension, such as an update retried from a ticker, starts afresh.
        if self.owner.as_deref() != Some(owner.as_str()) {
            self.stack.clear();
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
            (outcome @ (ActionOutcome::CopyToClipboard(_) | ActionOutcome::Copied), _) => {
                if let ActionOutcome::CopyToClipboard(text) = outcome {
                    fx.push(Effect::CopyText(text));
                }
                fx.push(Effect::Dismiss);
                self.show_notice(TickerStyle::Success, "Copied to clipboard".into(), None, fx);
            }
            // A root action that produces a view opens it. A snippet needing
            // arguments has to ask before it can do anything.
            (ActionOutcome::Replaced(tree), Origin::Root) => {
                if self.refused(&tree) {
                    return;
                }
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
                if self.refused(&tree) {
                    return;
                }
                if self.stack.is_empty() {
                    self.push(*tree, Context::default(), fx);
                } else {
                    self.replace_top(*tree, fx);
                }
            }
            (ActionOutcome::Back, Origin::View) => {
                if !self.stack.is_empty() {
                    self.escape(fx);
                }
            }
            (ActionOutcome::Back, Origin::Root) => {}
            (ActionOutcome::Ticker(text), _) => self.show_success(text, None, fx),
            (ActionOutcome::Failed(message), _) => self.show_failure(message, fx),
        }
    }

    fn refused(&mut self, tree: &ViewTree) -> bool {
        let Err(error) = check(tree) else {
            return false;
        };
        self.view_error = Some(error);
        true
    }

    /// Off screen nobody would see a ticker, and it would be stale by the next
    /// activation, so the notice says it instead.
    fn show_failure(&mut self, text: String, fx: &mut Vec<Effect>) {
        if !self.visible {
            self.show_notice(TickerStyle::Failure, text, None, fx);
            return;
        }
        let id = self.next_revision();
        self.ticker = Some(HeldTicker {
            id,
            style: TickerStyle::Failure,
            text,
            action: None,
        });
    }

    /// The ticker's action, while the ticker shows and the panel does not own
    /// the keyboard.
    pub(super) fn ticker_action(&self) -> Option<TickerAction> {
        if self.panel.is_some() {
            return None;
        }
        match &self.ticker {
            Some(ticker) => ticker.action.clone(),
            None => self.busy_cancel(),
        }
    }

    /// A ticker that waits for the user, because it failed or offers an
    /// action, goes with their next key press or click.
    fn clear_lasting(&mut self) {
        if self
            .ticker
            .as_ref()
            .is_some_and(|ticker| ticker.style == TickerStyle::Failure || ticker.action.is_some())
        {
            self.ticker = None;
        }
    }

    fn show_success(&mut self, text: String, action: Option<TickerAction>, fx: &mut Vec<Effect>) {
        let id = self.next_revision();
        let lasts = action.is_some();
        self.ticker = Some(HeldTicker {
            id,
            style: TickerStyle::Success,
            text,
            action,
        });
        if !lasts {
            fx.push(Effect::ExpireTicker {
                id,
                style: TickerStyle::Success,
            });
        }
    }

    /// The time runs from when the notice can be seen, so one set while the
    /// launcher is still on screen starts its time at the hide.
    /// An action on the notice is not drawn: opening the launcher while it
    /// shows runs it.
    fn show_notice(
        &mut self,
        style: TickerStyle,
        text: String,
        action: Option<TickerAction>,
        fx: &mut Vec<Effect>,
    ) {
        let id = self.next_revision();
        self.notice = Some(HeldTicker {
            id,
            style,
            text,
            action,
        });
        if !self.visible {
            fx.push(Effect::ExpireTicker { id, style });
        }
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
                if text.is_empty() {
                    self.root_open = false;
                }
                // The empty-query list is no partial answer for a typed query,
                // nor the other way round.
                if self.query.is_empty() != text.is_empty() {
                    self.results.clear();
                    self.typed_card = false;
                }
                self.query = text.clone();
                self.results_complete = false;
                self.picked = None;
                self.clear_lasting();
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
                        self.panel_selected(&actions),
                        down,
                    );
                    if let Some(panel) = self.panel.as_mut() {
                        panel.picked = next;
                    }
                } else if enter {
                    if let Some(id) = self.panel_selected(&self.panel_actions()) {
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
            if down && self.root_closed() {
                let bare = self.root_order().next().is_none();
                self.root_open = true;
                if bare {
                    self.picked = None;
                    return;
                }
            }
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
        self.clear_lasting();
        if !self.stack.is_empty() {
            // A command may still be working behind this view, and the user has
            // left it: stop it rather than paying for an answer nobody reads.
            fx.push(Effect::CancelInvocation);
            self.abandoned_invocation = self.last_invocation;
            self.shown_invocation = None;
            self.stack.pop();
            self.busy_marks = BusyMarks::Hidden;
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
            self.root_open = false;
            fx.push(Effect::Search(String::new()));
        } else if self.root_open {
            self.root_open = false;
            self.picked = None;
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
                        self.clear_lasting();
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
        self.clear_lasting();
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
        self.clear_lasting();
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
            Some(View::Detail(_) | View::Progress(_)) => Mode::Detail,
            Some(View::Form(_)) => Mode::Form,
        }
    }

    /// Root results in the order they are drawn: the answer, pinned, then
    /// suggestions, then everything else.
    pub(super) fn root_order(&self) -> impl Iterator<Item = &ResultItem> {
        let answer = self.results.iter().filter(|item| item.answer);
        let rows = || {
            self.results
                .iter()
                .filter(|item| !item.answer && self.root_shows(item))
        };
        let pinned = rows().filter(|item| item.pinned);
        let suggested = rows().filter(|item| !item.pinned && item.suggested);
        let rest = rows().filter(|item| !item.pinned && !item.suggested);
        answer.chain(pinned).chain(suggested).chain(rest)
    }

    pub(super) fn root_selected(&self) -> Option<&ResultItem> {
        self.picked
            .as_deref()
            .and_then(|picked| self.root_order().find(|item| item.id == picked))
            .or_else(|| self.root_order().next())
    }

    pub(super) fn root_closed(&self) -> bool {
        self.query.is_empty() && !self.root_open
    }

    /// Whether root shows the item: a closed root shows pinned items only.
    pub(super) fn root_shows(&self, item: &ResultItem) -> bool {
        item.pinned || !self.root_closed()
    }

    /// A pushed list's items after launcher-side narrowing, grouped in section
    /// order, without the groups left empty. Items naming no declared section
    /// come first.
    pub(super) fn view_sections(&self) -> Vec<(Option<&Section>, Vec<&ListItem>)> {
        let Some(view) = self.stack.last() else {
            return Vec::new();
        };
        let View::List(list) = &view.tree.view else {
            return Vec::new();
        };
        group(&list.items, &list.sections, &view.query)
    }

    /// A pushed grid's tiles, grouped and narrowed as a list's items are.
    pub(super) fn grid_sections(&self) -> Vec<(Option<&Section>, Vec<&GridItem>)> {
        let Some(view) = self.stack.last() else {
            return Vec::new();
        };
        let View::Grid(grid) = &view.tree.view else {
            return Vec::new();
        };
        group(&grid.items, &grid.sections, &view.query)
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
        self.pick_entry(&self.entries())
    }

    pub(super) fn pick_entry<'a>(&self, entries: &[Entry<'a>]) -> Option<Entry<'a>> {
        let picked = self.stack.last().and_then(|view| view.picked.as_deref());
        picked
            .and_then(|picked| entries.iter().find(|entry| entry.id() == picked))
            .or_else(|| entries.first())
            .copied()
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
            Some(View::Progress(progress)) => &progress.actions,
            Some(View::Form(form)) => &form.actions,
            Some(View::List(_) | View::Grid(_)) | None => &[],
        }
    }

    /// What the action panel offers. A list item's own actions win over the
    /// view's, because the item is what the user has selected.
    pub(super) fn available_actions(&self) -> &[Action] {
        self.actions_for(self.entry_selected())
    }

    pub(super) fn actions_for<'a>(&'a self, selected: Option<Entry<'a>>) -> &'a [Action] {
        match self.surface() {
            Mode::Root => self.root_selected().map_or(&[], |item| &item.actions),
            Mode::List | Mode::Grid => match selected {
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
        self.panel_actions_for(self.available_actions())
    }

    pub(super) fn panel_actions_for(&self, actions: &[Action]) -> Vec<Action> {
        let ordered = ordered_actions(actions);
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

    pub(super) fn panel_selected(&self, actions: &[Action]) -> Option<String> {
        let panel = self.panel.as_ref()?;
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
            Some(View::Detail(_) | View::Progress(_)) | None => None,
        }
    }
}

fn ordered_actions(actions: &[Action]) -> Vec<&Action> {
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

/// Marks a panel entry as a filter option rather than an action.
pub(super) const FILTER_PREFIX: &str = "dango.filter:";

/// Items in section order, narrowed by the query and then cut to the item
/// limit, without the groups left empty. Items naming no declared section come
/// first.
fn group<'a, T: Grouped>(
    items: &'a [T],
    sections: &'a [Section],
    query: &str,
) -> Vec<(Option<&'a Section>, Vec<&'a T>)> {
    let needle = query.to_lowercase();
    let shown = |item: &&T| {
        needle.is_empty()
            || item.title().to_lowercase().contains(&needle)
            || item
                .keywords()
                .iter()
                .any(|keyword| keyword.to_lowercase().contains(&needle))
    };
    let declared = |id: &str| sections.iter().any(|section| section.id == id);
    let loose: Vec<_> = items
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
    let mut left = super::ITEM_LIMIT;
    for (_, members) in &mut groups {
        members.truncate(left);
        left -= members.len();
    }
    groups.retain(|(_, members): &(_, Vec<_>)| !members.is_empty());
    groups
}

fn check(tree: &ViewTree) -> Result<(), ViewError> {
    if tree.protocol_version != PROTOCOL_VERSION {
        return Err(ViewError::Unsupported);
    }
    if matches!(&tree.view, View::Grid(grid) if !GRID_COLUMNS.contains(&grid.columns)) {
        return Err(ViewError::Invalid);
    }
    let mut shortcuts = tree_actions(&tree.view)
        .into_iter()
        .filter_map(|action| action.shortcut.as_ref());
    if !shortcuts.all(fires) {
        return Err(ViewError::Invalid);
    }
    Ok(())
}

fn tree_actions(view: &View) -> Vec<&Action> {
    match view {
        View::List(list) => list
            .items
            .iter()
            .flat_map(|item| &item.actions)
            .chain(list.empty_state.iter().flat_map(|state| &state.actions))
            .collect(),
        View::Grid(grid) => grid
            .items
            .iter()
            .flat_map(|item| &item.actions)
            .chain(grid.empty_state.iter().flat_map(|state| &state.actions))
            .collect(),
        View::Detail(detail) => detail.actions.iter().collect(),
        View::Progress(progress) => progress.actions.iter().collect(),
        View::Form(form) => form.actions.iter().collect(),
    }
}

const NAMED_KEYS: [&str; 13] = [
    "backspace",
    "tab",
    "enter",
    "escape",
    "delete",
    "arrowup",
    "arrowdown",
    "arrowleft",
    "arrowright",
    "home",
    "end",
    "pageup",
    "pagedown",
];

fn fires(shortcut: &Shortcut) -> bool {
    let shifted = shortcut
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier, Modifier::Shift | Modifier::Alt));
    let key = &shortcut.key;
    let letter = key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic();
    let named = NAMED_KEYS.iter().any(|name| key.eq_ignore_ascii_case(name));
    !shifted || letter || named
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

fn is_loading(view: &View) -> bool {
    match view {
        View::List(list) => list.loading,
        View::Grid(grid) => grid.loading,
        View::Detail(detail) => detail.loading,
        View::Form(_) | View::Progress(_) => false,
    }
}

fn show_answer(task: &TaskState) -> Option<TickerAction> {
    task.show
        .as_deref()
        .map(|show| task_action(task, "Show answer", "Enter", show))
}

/// An action of a task's owner, offered on a ticker with the platform's
/// command key.
fn task_action(task: &TaskState, title: &str, key: &str, action_id: &str) -> TickerAction {
    TickerAction {
        title: title.into(),
        shortcut: Shortcut {
            key: key.into(),
            modifiers: vec![Modifier::Cmd],
        },
        request: ActionRequest {
            origin: Origin::Root,
            extension_id: task.owner.clone(),
            item_id: task.item_id.clone(),
            action_id: action_id.into(),
            values: None,
            query: None,
        },
    }
}
