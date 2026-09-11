//! The root search pipeline: each keystroke cancels the previous query and
//! starts a new one, gathers candidates from the command registry and every
//! enabled root items provider, and streams merged snapshots as providers
//! answer. A slow or hung provider never blocks the rest.

use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout;

/// Where a candidate came from. Commands are static; root items are produced
/// per keystroke by a provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Command,
    RootItem,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub keywords: Vec<String>,
    pub alias: Option<String>,
    pub source: Source,
    /// Character offsets in the title that matched the query, filled by the
    /// ranker so the frontend can highlight them. Empty until ranked.
    pub match_positions: Vec<usize>,
}

/// A merged, ranked, bounded snapshot for one query. Emitted repeatedly as
/// providers answer; each is a full replacement, never a delta.
#[derive(Clone, Debug)]
pub struct SearchResults {
    pub query: String,
    pub items: Vec<Candidate>,
    pub complete: bool,
}

/// The static commands from the registry, snapshotted per query so no lock is
/// held across an await.
pub trait CommandSource: Send + Sync {
    fn candidates(&self) -> Vec<Candidate>;
}

/// One extension's per-keystroke provider. Cancellation is by dropping the
/// future, which happens when a newer query aborts the running one.
#[async_trait]
pub trait RootProvider: Send + Sync {
    async fn items(&self, query: String) -> Vec<Candidate>;
}

/// Orders and bounds the merged candidates for a query. The real matcher and
/// frecency live in the ranking module; the pipeline only needs this shape.
pub trait Ranker: Send + Sync {
    fn rank(&self, query: &str, candidates: Vec<Candidate>, limit: usize) -> Vec<Candidate>;
}

pub struct SearchPipeline {
    commands: std::sync::Arc<dyn CommandSource>,
    providers: Vec<std::sync::Arc<dyn RootProvider>>,
    ranker: std::sync::Arc<dyn Ranker>,
    /// A provider that has not answered by this point is abandoned so a hung one
    /// cannot keep the query alive forever. The 50ms responsiveness budget is
    /// met structurally by streaming, not by discarding here.
    straggler_timeout: Duration,
    limit: usize,
    current: Mutex<Option<JoinHandle<()>>>,
}

impl SearchPipeline {
    pub fn new(
        commands: std::sync::Arc<dyn CommandSource>,
        providers: Vec<std::sync::Arc<dyn RootProvider>>,
        ranker: std::sync::Arc<dyn Ranker>,
        limit: usize,
    ) -> Self {
        Self {
            commands,
            providers,
            ranker,
            straggler_timeout: Duration::from_secs(5),
            limit,
            current: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_straggler_timeout(mut self, timeout: Duration) -> Self {
        self.straggler_timeout = timeout;
        self
    }

    /// Cancels any in-flight query and starts a new one. The returned receiver
    /// yields merged snapshots until the query completes. There is no debounce
    /// and at most one query runs at a time.
    pub fn query(&self, query: String) -> mpsc::UnboundedReceiver<SearchResults> {
        let (tx, rx) = mpsc::unbounded_channel();

        if let Some(previous) = self.current.lock().unwrap().take() {
            previous.abort();
        }

        let commands = self.commands.candidates();
        let providers = self.providers.clone();
        let ranker = self.ranker.clone();
        let straggler_timeout = self.straggler_timeout;
        let limit = self.limit;

        let handle = tokio::spawn(async move {
            run_query(
                query,
                commands,
                providers,
                ranker,
                straggler_timeout,
                limit,
                tx,
            )
            .await;
        });

        *self.current.lock().unwrap() = Some(handle);
        rx
    }
}

async fn run_query(
    query: String,
    commands: Vec<Candidate>,
    providers: Vec<std::sync::Arc<dyn RootProvider>>,
    ranker: std::sync::Arc<dyn Ranker>,
    straggler_timeout: Duration,
    limit: usize,
    tx: mpsc::UnboundedSender<SearchResults>,
) {
    let mut merged = commands;
    let emit = |merged: &[Candidate], complete: bool| {
        let ranked = ranker.rank(&query, merged.to_vec(), limit);
        tx.send(SearchResults {
            query: query.clone(),
            items: ranked,
            complete,
        })
        .is_ok()
    };

    // Commands are available immediately; show them before any provider answers.
    if !emit(&merged, providers.is_empty()) {
        return;
    }

    let mut set = JoinSet::new();
    for provider in providers {
        let query = query.clone();
        set.spawn(async move {
            timeout(straggler_timeout, provider.items(query))
                .await
                .unwrap_or_default()
        });
    }

    while let Some(joined) = set.join_next().await {
        // A provider task that panicked is skipped; the rest keep streaming.
        if let Ok(items) = joined {
            merged.extend(items);
        }
        if !emit(&merged, set.is_empty()) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn candidate(id: &str, source: Source) -> Candidate {
        Candidate {
            id: id.into(),
            title: id.into(),
            subtitle: None,
            icon: None,
            keywords: vec![],
            alias: None,
            source,
            match_positions: vec![],
        }
    }

    struct Commands(Vec<Candidate>);
    impl CommandSource for Commands {
        fn candidates(&self) -> Vec<Candidate> {
            self.0.clone()
        }
    }

    /// Substring match on the title, insertion order preserved. Real ranking is
    /// the ranking module's job.
    struct SubstringRanker;
    impl Ranker for SubstringRanker {
        fn rank(&self, query: &str, candidates: Vec<Candidate>, limit: usize) -> Vec<Candidate> {
            candidates
                .into_iter()
                .filter(|c| c.title.to_lowercase().contains(&query.to_lowercase()))
                .take(limit)
                .collect()
        }
    }

    struct SleepyProvider {
        id: String,
        delay: Duration,
    }

    impl SleepyProvider {
        fn named(id: &str, delay_ms: u64) -> Self {
            Self {
                id: id.into(),
                delay: Duration::from_millis(delay_ms),
            }
        }
    }

    #[async_trait]
    impl RootProvider for SleepyProvider {
        async fn items(&self, _query: String) -> Vec<Candidate> {
            tokio::time::sleep(self.delay).await;
            vec![candidate(&self.id, Source::RootItem)]
        }
    }

    fn pipeline(providers: Vec<Arc<dyn RootProvider>>) -> SearchPipeline {
        SearchPipeline::new(
            Arc::new(Commands(vec![candidate("Calculator", Source::Command)])),
            providers,
            Arc::new(SubstringRanker),
            100,
        )
    }

    #[tokio::test]
    async fn commands_appear_before_any_provider_answers() {
        let slow: Arc<dyn RootProvider> = Arc::new(SleepyProvider::named("slow", 200));
        let pipe = pipeline(vec![slow]);
        let mut rx = pipe.query("cal".into());

        let first = rx.recv().await.unwrap();
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].id, "Calculator");
        assert!(!first.complete);
    }

    #[tokio::test]
    async fn a_slow_provider_does_not_delay_a_fast_one() {
        let fast: Arc<dyn RootProvider> = Arc::new(SleepyProvider::named("Fast", 5));
        let slow: Arc<dyn RootProvider> = Arc::new(SleepyProvider::named("Slow", 300));
        let pipe = pipeline(vec![fast, slow]);
        let mut rx = pipe.query("s".into());

        let mut seen_fast = false;
        let mut seen_slow = false;
        while let Some(update) = rx.recv().await {
            if update.items.iter().any(|c| c.id == "Fast") {
                seen_fast = true;
            }
            if update.items.iter().any(|c| c.id == "Slow") {
                seen_slow = true;
            }
            if update.complete {
                break;
            }
        }
        assert!(seen_fast && seen_slow);
    }

    #[tokio::test]
    async fn a_never_returning_provider_is_abandoned() {
        struct Pending;
        #[async_trait]
        impl RootProvider for Pending {
            async fn items(&self, _query: String) -> Vec<Candidate> {
                std::future::pending().await
            }
        }
        let good: Arc<dyn RootProvider> = Arc::new(SleepyProvider::named("Good", 5));
        let pipe = pipeline(vec![Arc::new(Pending), good])
            .with_straggler_timeout(Duration::from_millis(150));
        let mut rx = pipe.query("g".into());

        let mut last = None;
        while let Some(update) = rx.recv().await {
            let complete = update.complete;
            last = Some(update);
            if complete {
                break;
            }
        }
        let last = last.unwrap();
        assert!(last.complete);
        assert!(last.items.iter().any(|c| c.id == "Good"));
    }

    #[tokio::test]
    async fn a_new_query_cancels_the_previous_one() {
        // Records, per query, whether that run completed or was dropped mid
        // flight, so the superseded run is distinguishable from the live one.
        type Events = Arc<std::sync::Mutex<Vec<(String, bool)>>>;

        struct Guard {
            query: String,
            done: bool,
            events: Events,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                self.events
                    .lock()
                    .unwrap()
                    .push((self.query.clone(), self.done));
            }
        }

        struct RecordingProvider {
            delay: Duration,
            events: Events,
        }
        #[async_trait]
        impl RootProvider for RecordingProvider {
            async fn items(&self, query: String) -> Vec<Candidate> {
                let mut guard = Guard {
                    query,
                    done: false,
                    events: self.events.clone(),
                };
                tokio::time::sleep(self.delay).await;
                guard.done = true;
                vec![candidate("Calculator", Source::RootItem)]
            }
        }

        let events: Events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let provider: Arc<dyn RootProvider> = Arc::new(RecordingProvider {
            delay: Duration::from_millis(200),
            events: events.clone(),
        });
        let pipe = pipeline(vec![provider]);

        let _stale = pipe.query("first".into());
        tokio::time::sleep(Duration::from_millis(20)).await;
        let mut live = pipe.query("second".into());

        // Drain the live query to completion.
        while let Some(update) = live.recv().await {
            if update.complete {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;

        let events = events.lock().unwrap();
        assert!(
            events.contains(&("first".into(), false)),
            "the superseded query should have been cancelled mid flight: {events:?}"
        );
        assert!(
            events.contains(&("second".into(), true)),
            "the live query should have completed: {events:?}"
        );
    }

    #[tokio::test]
    async fn the_result_list_is_bounded() {
        let commands: Vec<Candidate> = (0..50)
            .map(|i| candidate(&format!("item{i}"), Source::Command))
            .collect();
        let pipe = SearchPipeline::new(
            Arc::new(Commands(commands)),
            vec![],
            Arc::new(SubstringRanker),
            10,
        );
        let mut rx = pipe.query("item".into());
        let update = rx.recv().await.unwrap();
        assert_eq!(update.items.len(), 10);
        assert!(update.complete);
    }
}
