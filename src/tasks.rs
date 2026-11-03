//! Long jobs that outlive the view that started them. Work reports through a
//! handle; every report reaches the launcher, which shows the job in its
//! progress view, as a busy toast elsewhere, or as a result toast later.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::watch;

use crate::protocol::{ProgressView, Status, Step, View, ViewTree};

pub type Notify = Box<dyn Fn(TaskState) + Send + Sync>;

pub struct Tasks {
    /// Oldest first.
    running: Mutex<Vec<TaskState>>,
    next: AtomicU64,
    notify: Notify,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskState {
    pub id: u64,
    /// The extension that runs it, whose actions cancel and retry it.
    pub owner: String,
    pub title: String,
    /// What the busy toast says while it runs.
    pub line: String,
    pub status: TaskStatus,
    /// From 0 to 1; none when the total is unknown.
    pub progress: Option<f32>,
    pub steps: Vec<Step>,
    pub subtitle: String,
    pub detail: String,
    /// The owner's action ids, run through the owner like any action.
    pub cancel: Option<String>,
    pub retry: Option<String>,
    /// Which of the owner's items the actions apply to.
    pub item_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Running,
    /// Waiting for the user, with what a toast says about it.
    NeedsInput {
        sentence: String,
    },
    Done {
        sentence: String,
    },
    Failed {
        sentence: String,
    },
    Cancelled,
}

impl TaskStatus {
    pub fn ended(&self) -> bool {
        matches!(
            self,
            TaskStatus::Done { .. } | TaskStatus::Failed { .. } | TaskStatus::Cancelled
        )
    }
}

impl Tasks {
    pub fn new(notify: impl Fn(TaskState) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            running: Mutex::new(Vec::new()),
            next: AtomicU64::new(1),
            notify: Box::new(notify),
        })
    }

    pub fn start(self: &Arc<Self>, owner: &str, item_id: &str, title: &str) -> TaskHandle {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let state = TaskState {
            id,
            owner: owner.into(),
            title: title.into(),
            line: title.into(),
            status: TaskStatus::Running,
            progress: None,
            steps: Vec::new(),
            subtitle: String::new(),
            detail: String::new(),
            cancel: None,
            retry: None,
            item_id: item_id.into(),
        };
        self.running.lock().unwrap().push(state.clone());
        (self.notify)(state);
        let (cancel, cancelled) = watch::channel(false);
        TaskHandle {
            id,
            tasks: self.clone(),
            cancel: Arc::new(cancel),
            cancelled,
        }
    }

    pub fn running(&self) -> Vec<TaskState> {
        self.running.lock().unwrap().clone()
    }
}

#[derive(Clone)]
pub struct TaskHandle {
    id: u64,
    tasks: Arc<Tasks>,
    cancel: Arc<watch::Sender<bool>>,
    cancelled: watch::Receiver<bool>,
}

impl TaskHandle {
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Changes the task and tells the launcher. An ended task leaves the
    /// registry, and later reports do nothing.
    pub fn report(&self, change: impl FnOnce(&mut TaskState)) {
        let state = {
            let mut running = self.tasks.running.lock().unwrap();
            let Some(index) = running.iter().position(|task| task.id == self.id) else {
                return;
            };
            change(&mut running[index]);
            let state = running[index].clone();
            if state.status.ended() {
                running.remove(index);
            }
            state
        };
        (self.tasks.notify)(state);
    }

    pub fn state(&self) -> Option<TaskState> {
        let running = self.tasks.running.lock().unwrap();
        running.iter().find(|task| task.id == self.id).cloned()
    }

    pub fn cancel(&self) {
        self.cancel.send_replace(true);
    }

    /// Changes to true once the task is cancelled.
    pub fn cancelled(&self) -> watch::Receiver<bool> {
        self.cancelled.clone()
    }
}

/// The task as the progress view a command pushes to show it.
pub fn progress_view(state: &TaskState, actions: Vec<crate::protocol::Action>) -> ViewTree {
    ViewTree::new(View::Progress(ProgressView {
        title: state.title.clone(),
        subtitle: state.subtitle.clone(),
        value: state.progress,
        detail: state.detail.clone(),
        steps: state.steps.clone(),
        actions,
    }))
}

/// Steps before `current` done, `current` in `status`, the rest queued.
pub fn steps(titles: &[&str], current: usize, status: Status) -> Vec<Step> {
    titles
        .iter()
        .enumerate()
        .map(|(index, title)| Step {
            title: (*title).into(),
            status: match index.cmp(&current) {
                std::cmp::Ordering::Less => Status::Done,
                std::cmp::Ordering::Equal => status,
                std::cmp::Ordering::Greater => Status::Queued,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded() -> (Arc<Tasks>, Arc<Mutex<Vec<TaskState>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let tasks = Tasks::new(move |state| sink.lock().unwrap().push(state));
        (tasks, seen)
    }

    #[test]
    fn reports_reach_the_launcher() {
        let (tasks, seen) = recorded();
        let task = tasks.start("dango.updates", "update", "Dango 0.5.0");
        task.report(|state| state.progress = Some(0.5));
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1].progress, Some(0.5));
    }

    #[test]
    fn cancel_flips_the_signal() {
        let (tasks, _) = recorded();
        let task = tasks.start("dango.updates", "update", "Dango 0.5.0");
        let cancelled = task.cancelled();
        assert!(!*cancelled.borrow());
        task.clone().cancel();
        assert!(*cancelled.borrow());
    }

    #[test]
    fn an_ended_task_leaves_and_needs_input_stays() {
        let (tasks, seen) = recorded();
        let waiting = tasks.start("dango.updates", "update", "Waiting");
        waiting.report(|state| {
            state.status = TaskStatus::NeedsInput {
                sentence: "Ready".into(),
            }
        });
        let failing = tasks.start("other", "", "Failing");
        failing.report(|state| {
            state.status = TaskStatus::Failed {
                sentence: "No".into(),
            }
        });
        failing.report(|state| state.progress = Some(1.0));
        let ids: Vec<u64> = tasks.running().iter().map(|task| task.id).collect();
        assert_eq!(ids, [waiting.id()]);
        assert_eq!(seen.lock().unwrap().len(), 4, "no report after the end");
    }

    #[test]
    fn the_progress_view_maps_the_state() {
        let (tasks, _) = recorded();
        let task = tasks.start("dango.updates", "update", "Dango 0.5.0");
        task.report(|state| {
            state.progress = Some(0.62);
            state.steps = steps(
                &["Download", "Verify signature", "Restart"],
                0,
                Status::Working,
            );
        });
        let View::Progress(view) = progress_view(&task.state().unwrap(), Vec::new()).view else {
            panic!("expected a progress view");
        };
        assert_eq!(view.value, Some(0.62));
        assert_eq!(view.title, "Dango 0.5.0");
        let statuses: Vec<Status> = view.steps.iter().map(|step| step.status).collect();
        assert_eq!(statuses, [Status::Working, Status::Queued, Status::Queued]);
    }
}
