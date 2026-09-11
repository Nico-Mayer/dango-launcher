//! Running a command. The registry says a command exists and the protocol says
//! what it can put on screen; this is the path between them.
//!
//! Invocation returns at once and output arrives on a channel, the same shape
//! the search pipeline uses, because a command that streams has to be able to
//! replace its view repeatedly while it is still working.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::extension::Host;
use crate::protocol::ViewTree;

/// What a no-view command produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure(String),
}

/// What an invocation sends back as it runs.
#[derive(Clone, Debug)]
pub enum Output {
    View(ViewTree),
    Finished(Outcome),
}

/// Where a running command puts what it produces. Behind this sits the check
/// that drops output from a superseded invocation.
pub trait Sink: Send + Sync {
    fn push_view(&self, tree: ViewTree);
    fn finish(&self, outcome: Outcome);
}

/// Handed to a command for the length of one invocation.
pub struct InvocationContext {
    pub command_id: String,
    sink: Arc<dyn Sink>,
}

impl InvocationContext {
    pub fn push_view(&self, tree: ViewTree) {
        self.sink.push_view(tree);
    }

    pub fn succeed(&self) {
        self.sink.finish(Outcome::Success);
    }

    pub fn fail(&self, message: impl Into<String>) {
        self.sink.finish(Outcome::Failure(message.into()));
    }
}

/// One invocable command. A built-in implements this directly; a sandboxed
/// extension will get an implementation that talks to its runtime.
pub trait Command: Send + Sync {
    fn invoke(&self, ctx: &InvocationContext);
}

pub struct Resolved {
    pub command: Arc<dyn Command>,
    pub host: Host,
}

/// Finds the code behind a qualified command identity. Separate from the
/// extension host so invocation can be tested without one.
pub trait CommandResolver: Send + Sync {
    fn resolve(&self, qualified_id: &str) -> Option<Resolved>;
}

/// Who actually executes a command. Every invocation goes through one, so the
/// sandboxed host M8 adds slots in without changing a caller.
pub trait CommandHost: Send + Sync {
    fn run(&self, command: Arc<dyn Command>, ctx: InvocationContext);
}

/// The native host: built-ins are Rust, so running one is calling it, off the
/// caller's thread because a command may block on the operating system.
pub struct BuiltinHost;

impl CommandHost for BuiltinHost {
    fn run(&self, command: Arc<dyn Command>, ctx: InvocationContext) {
        std::thread::spawn(move || {
            // Only catches an unwind, which a release build does not do because
            // the profile aborts. It still keeps a debug session alive.
            if catch_unwind(AssertUnwindSafe(|| command.invoke(&ctx))).is_err() {
                ctx.fail("the command crashed");
            }
        });
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvokeError {
    #[error("that command is no longer available")]
    Unavailable,
    #[error("that command cannot run in this build")]
    HostUnavailable,
}

/// Starts invocations and keeps at most one of them relevant. Superseding does
/// not interrupt the running command, which may be mid-way through an operating
/// system call that cannot be abandoned safely; it stops its output being
/// delivered, which is what the user can observe.
pub struct Invoker {
    resolver: Arc<dyn CommandResolver>,
    hosts: HashMap<Host, Arc<dyn CommandHost>>,
    generation: Arc<AtomicU64>,
}

impl Invoker {
    pub fn new(resolver: Arc<dyn CommandResolver>) -> Self {
        Self::with_hosts(
            resolver,
            [(Host::Builtin, Arc::new(BuiltinHost) as Arc<dyn CommandHost>)].into(),
        )
    }

    pub fn with_hosts(
        resolver: Arc<dyn CommandResolver>,
        hosts: HashMap<Host, Arc<dyn CommandHost>>,
    ) -> Self {
        Self {
            resolver,
            hosts,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn invoke(
        &self,
        qualified_id: &str,
    ) -> Result<mpsc::UnboundedReceiver<Output>, InvokeError> {
        let resolved = self
            .resolver
            .resolve(qualified_id)
            .ok_or(InvokeError::Unavailable)?;
        let host = self
            .hosts
            .get(&resolved.host)
            .cloned()
            .ok_or(InvokeError::HostUnavailable)?;

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (tx, rx) = mpsc::unbounded_channel();
        let ctx = InvocationContext {
            command_id: qualified_id.to_string(),
            sink: Arc::new(ChannelSink {
                generation,
                current: self.generation.clone(),
                tx,
            }),
        };
        host.run(resolved.command, ctx);
        Ok(rx)
    }

    /// Makes whatever is running irrelevant, so its later output is dropped.
    /// Used when the launcher hides with a command still working.
    pub fn abandon(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

struct ChannelSink {
    generation: u64,
    current: Arc<AtomicU64>,
    tx: mpsc::UnboundedSender<Output>,
}

impl ChannelSink {
    fn superseded(&self) -> bool {
        self.current.load(Ordering::SeqCst) != self.generation
    }
}

impl Sink for ChannelSink {
    fn push_view(&self, tree: ViewTree) {
        if !self.superseded() {
            let _ = self.tx.send(Output::View(tree));
        }
    }

    fn finish(&self, outcome: Outcome) {
        if !self.superseded() {
            let _ = self.tx.send(Output::Finished(outcome));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Filtering, ListView, View};
    use std::sync::Mutex;

    fn tree() -> ViewTree {
        ViewTree {
            protocol_version: crate::protocol::PROTOCOL_VERSION,
            view: View::List(ListView {
                filtering: Filtering::Launcher,
                loading: false,
                empty_state: None,
                items: vec![],
            }),
        }
    }

    struct Fixed(Mutex<HashMap<String, Resolved>>);

    impl Fixed {
        fn new(entries: Vec<(&str, Arc<dyn Command>, Host)>) -> Arc<Self> {
            Arc::new(Self(Mutex::new(
                entries
                    .into_iter()
                    .map(|(id, command, host)| (id.to_string(), Resolved { command, host }))
                    .collect(),
            )))
        }
    }

    impl CommandResolver for Fixed {
        fn resolve(&self, qualified_id: &str) -> Option<Resolved> {
            let map = self.0.lock().unwrap();
            map.get(qualified_id).map(|r| Resolved {
                command: r.command.clone(),
                host: r.host,
            })
        }
    }

    struct Succeeds;
    impl Command for Succeeds {
        fn invoke(&self, ctx: &InvocationContext) {
            ctx.succeed();
        }
    }

    struct Fails;
    impl Command for Fails {
        fn invoke(&self, ctx: &InvocationContext) {
            ctx.fail("no good");
        }
    }

    struct Panics;
    impl Command for Panics {
        fn invoke(&self, _ctx: &InvocationContext) {
            panic!("boom");
        }
    }

    struct PushesTwoTrees;
    impl Command for PushesTwoTrees {
        fn invoke(&self, ctx: &InvocationContext) {
            ctx.push_view(tree());
            ctx.push_view(tree());
            ctx.succeed();
        }
    }

    /// Records which command ran, so a test can tell two identically named
    /// commands apart.
    struct Names(&'static str, Arc<Mutex<Vec<&'static str>>>);
    impl Command for Names {
        fn invoke(&self, ctx: &InvocationContext) {
            self.1.lock().unwrap().push(self.0);
            ctx.succeed();
        }
    }

    /// Waits to be released, so a test can supersede it mid-flight.
    struct Blocks(Arc<Mutex<bool>>);
    impl Command for Blocks {
        fn invoke(&self, ctx: &InvocationContext) {
            while !*self.0.lock().unwrap() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            ctx.push_view(tree());
            ctx.succeed();
        }
    }

    #[tokio::test]
    async fn a_builtin_command_runs_through_the_host() {
        let invoker = Invoker::new(Fixed::new(vec![(
            "sys.lock",
            Arc::new(Succeeds),
            Host::Builtin,
        )]));
        let mut rx = invoker.invoke("sys.lock").unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(Output::Finished(Outcome::Success))
        ));
    }

    #[tokio::test]
    async fn a_command_can_push_more_than_one_tree() {
        let invoker = Invoker::new(Fixed::new(vec![(
            "sys.list",
            Arc::new(PushesTwoTrees),
            Host::Builtin,
        )]));
        let mut rx = invoker.invoke("sys.list").unwrap();
        assert!(matches!(rx.recv().await, Some(Output::View(_))));
        assert!(matches!(rx.recv().await, Some(Output::View(_))));
        assert!(matches!(rx.recv().await, Some(Output::Finished(_))));
    }

    #[test]
    fn a_command_naming_a_host_this_build_lacks_is_refused() {
        let invoker = Invoker::with_hosts(
            Fixed::new(vec![("sys.lock", Arc::new(Succeeds), Host::Builtin)]),
            HashMap::new(),
        );
        assert_eq!(
            invoker.invoke("sys.lock").unwrap_err(),
            InvokeError::HostUnavailable
        );
    }

    #[tokio::test]
    async fn the_qualified_identity_picks_between_two_extensions() {
        let ran = Arc::new(Mutex::new(Vec::new()));
        let invoker = Invoker::new(Fixed::new(vec![
            (
                "sys.quit",
                Arc::new(Names("system", ran.clone())),
                Host::Builtin,
            ),
            (
                "apps.quit",
                Arc::new(Names("applications", ran.clone())),
                Host::Builtin,
            ),
        ]));
        let mut rx = invoker.invoke("apps.quit").unwrap();
        rx.recv().await;
        assert_eq!(*ran.lock().unwrap(), ["applications"]);
    }

    #[test]
    fn an_unknown_command_is_unavailable_rather_than_a_panic() {
        let invoker = Invoker::new(Fixed::new(vec![]));
        assert_eq!(
            invoker.invoke("sys.gone").unwrap_err(),
            InvokeError::Unavailable
        );
    }

    #[tokio::test]
    async fn a_newer_invocation_supersedes_an_older_one() {
        let release = Arc::new(Mutex::new(false));
        let invoker = Invoker::new(Fixed::new(vec![
            ("sys.slow", Arc::new(Blocks(release.clone())), Host::Builtin),
            ("sys.quick", Arc::new(Succeeds), Host::Builtin),
        ]));

        let mut slow = invoker.invoke("sys.slow").unwrap();
        let mut quick = invoker.invoke("sys.quick").unwrap();
        assert!(matches!(
            quick.recv().await,
            Some(Output::Finished(Outcome::Success))
        ));

        *release.lock().unwrap() = true;
        // The superseded command runs to completion; nothing it produced is
        // delivered, and its channel closes empty.
        assert!(slow.recv().await.is_none());
    }

    #[tokio::test]
    async fn abandoning_drops_the_output_of_a_running_command() {
        let release = Arc::new(Mutex::new(false));
        let invoker = Invoker::new(Fixed::new(vec![(
            "sys.slow",
            Arc::new(Blocks(release.clone())),
            Host::Builtin,
        )]));
        let mut rx = invoker.invoke("sys.slow").unwrap();
        invoker.abandon();
        *release.lock().unwrap() = true;
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn a_failing_command_reports_a_failure() {
        let invoker = Invoker::new(Fixed::new(vec![("sys.x", Arc::new(Fails), Host::Builtin)]));
        let mut rx = invoker.invoke("sys.x").unwrap();
        assert!(
            matches!(rx.recv().await, Some(Output::Finished(Outcome::Failure(m))) if m == "no good")
        );
    }

    #[tokio::test]
    async fn a_panicking_command_becomes_a_failure() {
        let invoker = Invoker::new(Fixed::new(vec![(
            "sys.boom",
            Arc::new(Panics),
            Host::Builtin,
        )]));
        let mut rx = invoker.invoke("sys.boom").unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(Output::Finished(Outcome::Failure(_)))
        ));
    }
}
