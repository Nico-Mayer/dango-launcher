pub mod manifest;
pub mod preferences;
pub mod registry;

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

pub use manifest::{
    CommandDecl, InvocationMode, Manifest, ManifestError, PreferenceDecl, PreferenceKind,
    NAMED_ICON,
};
pub use preferences::{PreferenceStore, Preferences};
pub use registry::{Collision, Host, RegisteredCommand, Registry};

pub type ActivationResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// What a submitted form carried, keyed by field id. Empty for an action run
/// on a list item, which is most of them.
pub type FormValues = std::collections::HashMap<String, String>;

/// What running an action on a result produced. `Done` means the work is
/// finished and the launcher should get out of the way; a copy hands text back
/// because only the webview can reach the clipboard.
#[derive(Debug, PartialEq)]
pub enum ActionOutcome {
    Done,
    /// The action started work that will report for itself, streaming its views
    /// through the launcher's render channel. Nothing to show yet, and the
    /// launcher stays where it is.
    Started,
    CopyToClipboard(String),
    /// The action changed what the view was showing, so the view is replaced
    /// and the user stays where they are. Removing one of a list of things is
    /// the case that needs it: hiding the launcher after each one would make
    /// clearing several a chore.
    Replaced(Box<crate::protocol::ViewTree>),
    Removed(Box<crate::protocol::ViewTree>),
    Failed(String),
}

/// A native built-in. Its manifest is data; its behaviour is Rust. Third-party
/// extensions will implement the same contract behind a sandboxed host later.
pub trait Extension: Send + Sync {
    fn manifest(&self) -> &Manifest;
    fn activate(&self) -> ActivationResult {
        Ok(())
    }
    fn deactivate(&self) {}
    /// Background work whose lifetime follows the enabled state. Started on
    /// activate, stopped on deactivate. The contribution type third-party
    /// extensions will never get.
    fn services(&self) -> Vec<Arc<dyn Service>> {
        Vec::new()
    }
    /// The single per-keystroke root items provider, if the extension has one.
    fn root_provider(&self) -> Option<Arc<dyn crate::search::RootProvider>> {
        None
    }
    /// The code behind one of the extension's declared commands, by its
    /// unqualified id. A declaration without an implementation is not
    /// invocable, which the host reports rather than treating as a crash.
    fn command(&self, _command_id: &str) -> Option<Arc<dyn crate::invocation::Command>> {
        None
    }
    /// Runs an action on a result this extension contributed. A form submit is
    /// an action too, and `values` is what its fields held.
    fn perform_action(
        &self,
        _item_id: &str,
        _action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        ActionOutcome::Failed("That action isn't available.".into())
    }
}

pub trait Service: Send + Sync {
    fn start(&self) -> ActivationResult {
        Ok(())
    }
    fn stop(&self) {}
}

/// Where enabled state lives across restarts. Abstracted so the host is
/// testable without a database.
pub trait EnabledStore: Send + Sync {
    fn is_enabled(&self, extension_id: &str) -> bool;
    fn set_enabled(&self, extension_id: &str, enabled: bool);
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("an extension with id '{0}' is already registered")]
    DuplicateExtension(String),
}

/// One extension failing to load, activate, or register is recorded here rather
/// than propagated, so the launcher and the other extensions keep working.
#[derive(Debug, Default)]
pub struct LoadReport {
    pub activation_errors: Vec<(String, String)>,
    pub collisions: Vec<Collision>,
}

impl LoadReport {
    pub fn is_clean(&self) -> bool {
        self.activation_errors.is_empty() && self.collisions.is_empty()
    }
}

pub struct ExtensionHost {
    extensions: HashMap<String, Arc<dyn Extension>>,
    /// Maps an active extension to the services it started, so deactivation
    /// stops exactly those.
    active: HashMap<String, Vec<Arc<dyn Service>>>,
    registry: Registry,
    enabled: Arc<dyn EnabledStore>,
}

impl ExtensionHost {
    pub fn new(enabled: Arc<dyn EnabledStore>) -> Self {
        Self {
            extensions: HashMap::new(),
            active: HashMap::new(),
            registry: Registry::new(),
            enabled,
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn is_active(&self, extension_id: &str) -> bool {
        self.active.contains_key(extension_id)
    }

    /// Root providers of every active extension, for the search pipeline.
    pub fn root_providers(&self) -> Vec<Arc<dyn crate::search::RootProvider>> {
        self.active
            .keys()
            .filter_map(|id| self.extensions.get(id)?.root_provider())
            .collect()
    }

    /// The tint every registered extension declared, keyed by extension id.
    /// Declared once and never changed at runtime, so the caller can hold the
    /// result rather than asking again per query.
    pub fn tints(&self) -> HashMap<String, String> {
        self.extensions
            .values()
            .filter_map(|extension| {
                let manifest = extension.manifest();
                Some((manifest.id.clone(), manifest.tint.clone()?))
            })
            .collect()
    }

    /// Finds the code behind a qualified command identity. A command whose
    /// extension was disabled is gone from the registry, so this answers `None`
    /// and the caller reports it as unavailable.
    pub fn resolve_command(&self, qualified_id: &str) -> Option<crate::invocation::Resolved> {
        let registered = self.registry.get(qualified_id)?;
        let extension = self.extensions.get(&registered.extension_id)?;
        Some(crate::invocation::Resolved {
            command: extension.command(&registered.decl.id)?,
            host: registered.host,
        })
    }

    /// Runs an action through the extension that contributed the result. No
    /// extension is the default handler.
    pub fn perform_action(
        &self,
        extension_id: &str,
        item_id: &str,
        action_id: &str,
        values: &FormValues,
    ) -> ActionOutcome {
        if !self.is_active(extension_id) {
            return ActionOutcome::Failed("That extension is turned off.".into());
        }
        match self.extensions.get(extension_id) {
            Some(extension) => extension.perform_action(item_id, action_id, values),
            None => ActionOutcome::Failed("That extension is turned off.".into()),
        }
    }

    /// Every registered command as a search candidate.
    pub fn command_candidates(&self) -> Vec<crate::search::Candidate> {
        self.registry
            .commands()
            .map(|command| crate::search::Candidate {
                extension_id: command.extension_id.clone(),
                id: command.qualified_id(),
                title: command.decl.title.clone(),
                subtitle: command.decl.subtitle.clone(),
                icon: command.decl.icon.clone(),
                keywords: command.decl.keywords.clone(),
                alias: command.decl.alias.clone(),
                source: crate::search::Source::Command,
                actions: Vec::new(),
                match_positions: Vec::new(),
                suggested: false,
            })
            .collect()
    }

    /// Adds an extension and activates it when its persisted state says
    /// enabled. A malformed manifest or a duplicate id is rejected up front;
    /// an error thrown during activation is isolated into the report.
    pub fn register(&mut self, extension: Arc<dyn Extension>) -> Result<LoadReport, HostError> {
        let id = extension.manifest().id.clone();
        extension.manifest().validate()?;
        if self.extensions.contains_key(&id) {
            return Err(HostError::DuplicateExtension(id));
        }
        self.extensions.insert(id.clone(), extension);

        let mut report = LoadReport::default();
        if self.enabled.is_enabled(&id) {
            self.activate(&id, &mut report);
        }
        Ok(report)
    }

    /// Swaps an extension for a newly built instance of itself, which is how an
    /// extension whose commands come from the configuration file follows an edit
    /// to it. The old one is deactivated first, so its commands are unregistered
    /// and its services stopped, and the new one is activated when the enabled
    /// store says it should be.
    pub fn replace(&mut self, extension: Arc<dyn Extension>) -> Result<LoadReport, HostError> {
        let id = extension.manifest().id.clone();
        extension.manifest().validate()?;
        self.deactivate(&id);
        self.extensions.insert(id.clone(), extension);

        let mut report = LoadReport::default();
        if self.enabled.is_enabled(&id) {
            self.activate(&id, &mut report);
        }
        Ok(report)
    }

    pub fn set_enabled(&mut self, extension_id: &str, enabled: bool) -> LoadReport {
        self.enabled.set_enabled(extension_id, enabled);
        let mut report = LoadReport::default();
        if enabled {
            self.activate(extension_id, &mut report);
        } else {
            self.deactivate(extension_id);
        }
        report
    }

    /// Brings every extension's active state in line with the enabled store,
    /// without persisting anything. Used on a config reload, where the store
    /// (the file) is already the new source of truth.
    pub fn reload_enabled(&mut self) -> LoadReport {
        let ids: Vec<String> = self.extensions.keys().cloned().collect();
        let mut report = LoadReport::default();
        for id in ids {
            match (self.enabled.is_enabled(&id), self.is_active(&id)) {
                (true, false) => self.activate(&id, &mut report),
                (false, true) => self.deactivate(&id),
                _ => {}
            }
        }
        report
    }

    fn activate(&mut self, extension_id: &str, report: &mut LoadReport) {
        if self.active.contains_key(extension_id) {
            return;
        }
        let Some(extension) = self.extensions.get(extension_id).cloned() else {
            return;
        };

        // catch_unwind so a panicking activation cannot take down startup, on
        // top of the Result the trait already returns.
        let outcome = catch_unwind(AssertUnwindSafe(|| extension.activate()));
        match outcome {
            Ok(Ok(())) => {
                let collisions = self.registry.register(
                    extension_id,
                    &extension.manifest().commands,
                    Host::Builtin,
                );
                report.collisions.extend(collisions);
                let services = self.start_services(extension_id, &extension, report);
                self.active.insert(extension_id.to_string(), services);
            }
            Ok(Err(error)) => report
                .activation_errors
                .push((extension_id.to_string(), error.to_string())),
            Err(_) => report
                .activation_errors
                .push((extension_id.to_string(), "activation panicked".into())),
        }
    }

    fn start_services(
        &self,
        extension_id: &str,
        extension: &Arc<dyn Extension>,
        report: &mut LoadReport,
    ) -> Vec<Arc<dyn Service>> {
        let mut started = Vec::new();
        for service in extension.services() {
            match catch_unwind(AssertUnwindSafe(|| service.start())) {
                Ok(Ok(())) => started.push(service),
                Ok(Err(error)) => report
                    .activation_errors
                    .push((extension_id.to_string(), error.to_string())),
                Err(_) => report
                    .activation_errors
                    .push((extension_id.to_string(), "service panicked".into())),
            }
        }
        started
    }

    fn deactivate(&mut self, extension_id: &str) {
        let Some(services) = self.active.remove(extension_id) else {
            return;
        };
        for service in services {
            let _ = catch_unwind(AssertUnwindSafe(|| service.stop()));
        }
        self.registry.unregister(extension_id);
        if let Some(extension) = self.extensions.get(extension_id) {
            let _ = catch_unwind(AssertUnwindSafe(|| extension.deactivate()));
        }
    }
}

/// Resolves through the live host, so enabling or disabling an extension takes
/// effect without rebuilding the invoker.
pub struct HostResolver(pub Arc<Mutex<ExtensionHost>>);

impl crate::invocation::CommandResolver for HostResolver {
    fn resolve(&self, qualified_id: &str) -> Option<crate::invocation::Resolved> {
        self.0.lock().ok()?.resolve_command(qualified_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryEnabled {
        disabled: Mutex<Vec<String>>,
    }

    impl EnabledStore for MemoryEnabled {
        fn is_enabled(&self, id: &str) -> bool {
            !self.disabled.lock().unwrap().iter().any(|d| d == id)
        }
        fn set_enabled(&self, id: &str, enabled: bool) {
            let mut disabled = self.disabled.lock().unwrap();
            disabled.retain(|d| d != id);
            if !enabled {
                disabled.push(id.to_string());
            }
        }
    }

    struct TestExtension {
        manifest: Manifest,
        activate_count: AtomicUsize,
        deactivated: AtomicBool,
        fail: bool,
    }

    impl TestExtension {
        fn new(id: &str, command_id: &str) -> Arc<Self> {
            Arc::new(Self {
                manifest: Manifest {
                    manifest_version: 1,
                    id: id.into(),
                    name: id.into(),
                    icon: None,
                    tint: None,
                    commands: vec![CommandDecl {
                        id: command_id.into(),
                        title: command_id.into(),
                        mode: InvocationMode::View,
                        subtitle: None,
                        icon: None,
                        keywords: vec![],
                        alias: None,
                    }],
                    preferences: vec![],
                    root_items: false,
                    services: false,
                },
                activate_count: AtomicUsize::new(0),
                deactivated: AtomicBool::new(false),
                fail: false,
            })
        }
    }

    impl Extension for TestExtension {
        fn manifest(&self) -> &Manifest {
            &self.manifest
        }
        fn activate(&self) -> ActivationResult {
            self.activate_count.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err("boom".into());
            }
            Ok(())
        }
        fn deactivate(&self) {
            self.deactivated.store(true, Ordering::SeqCst);
        }
    }

    fn host() -> ExtensionHost {
        ExtensionHost::new(Arc::new(MemoryEnabled::default()))
    }

    #[test]
    fn reload_enabled_syncs_active_state_to_the_store() {
        let enabled = Arc::new(MemoryEnabled::default());
        let mut host = ExtensionHost::new(enabled.clone());
        let extension = TestExtension::new("apps", "open");
        host.register(extension.clone()).unwrap();
        assert!(host.is_active("apps"));

        // An external edit disables it; a reload brings the runtime in line
        // without the host calling set_enabled to persist anything back.
        enabled.set_enabled("apps", false);
        host.reload_enabled();
        assert!(!host.is_active("apps"));
        assert!(host.registry().get("apps.open").is_none());

        enabled.set_enabled("apps", true);
        host.reload_enabled();
        assert!(host.is_active("apps"));
        assert!(host.registry().get("apps.open").is_some());
    }

    #[test]
    fn registering_an_enabled_extension_activates_and_registers_commands() {
        let mut host = host();
        let report = host.register(TestExtension::new("apps", "open")).unwrap();
        assert!(report.is_clean());
        assert!(host.is_active("apps"));
        assert!(host.registry().get("apps.open").is_some());
    }

    #[test]
    fn disabling_removes_commands_and_stops_the_extension() {
        let mut host = host();
        let extension = TestExtension::new("apps", "open");
        host.register(extension.clone()).unwrap();
        host.set_enabled("apps", false);
        assert!(!host.is_active("apps"));
        assert!(host.registry().get("apps.open").is_none());
        assert!(extension.deactivated.load(Ordering::SeqCst));
    }

    #[test]
    fn re_enabling_restores_commands_without_reconstruction() {
        let mut host = host();
        let extension = TestExtension::new("apps", "open");
        host.register(extension.clone()).unwrap();
        host.set_enabled("apps", false);
        host.set_enabled("apps", true);
        assert!(host.registry().get("apps.open").is_some());
        assert_eq!(extension.activate_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn replacing_an_extension_swaps_its_commands_live() {
        let mut host = host();
        host.register(TestExtension::new("ai", "improve")).unwrap();
        assert!(host.registry().get("ai.improve").is_some());

        host.replace(TestExtension::new("ai", "translate")).unwrap();
        assert!(
            host.registry().get("ai.improve").is_none(),
            "the old command outlived its extension"
        );
        assert!(host.registry().get("ai.translate").is_some());
        assert!(host.is_active("ai"));
    }

    #[test]
    fn replacing_a_disabled_extension_leaves_it_disabled() {
        let enabled = Arc::new(MemoryEnabled::default());
        enabled.set_enabled("ai", false);
        let mut host = ExtensionHost::new(enabled);
        host.register(TestExtension::new("ai", "improve")).unwrap();

        host.replace(TestExtension::new("ai", "translate")).unwrap();
        assert!(!host.is_active("ai"));
        assert!(host.registry().is_empty(), "a disabled extension registered");
    }

    #[test]
    fn replacing_stops_the_old_instances_services() {
        let watcher = Arc::new(Watcher::default());
        let extension = Arc::new(ServiceExtension {
            manifest: Manifest {
                manifest_version: 1,
                id: "svc".into(),
                name: "svc".into(),
                icon: None,
                tint: None,
                commands: vec![],
                preferences: vec![],
                root_items: false,
                services: true,
            },
            watcher: watcher.clone(),
        });
        let mut host = host();
        host.register(extension).unwrap();
        assert!(watcher.running.load(Ordering::SeqCst));

        host.replace(TestExtension::new("svc", "x")).unwrap();
        assert!(
            !watcher.running.load(Ordering::SeqCst),
            "the old instance's service kept running"
        );
    }

    #[test]
    fn a_disabled_extension_is_not_activated_on_register() {
        let enabled = Arc::new(MemoryEnabled::default());
        enabled.set_enabled("apps", false);
        let mut host = ExtensionHost::new(enabled);
        host.register(TestExtension::new("apps", "open")).unwrap();
        assert!(!host.is_active("apps"));
        assert!(host.registry().is_empty());
    }

    #[test]
    fn a_failing_extension_is_isolated_and_others_still_load() {
        let mut host = host();
        let mut broken = TestExtension::new("broken", "x");
        Arc::get_mut(&mut broken).unwrap().fail = true;
        let report = host.register(broken).unwrap();
        assert_eq!(report.activation_errors.len(), 1);
        assert!(!host.is_active("broken"));

        let report = host.register(TestExtension::new("apps", "open")).unwrap();
        assert!(report.is_clean());
        assert!(host.registry().get("apps.open").is_some());
    }

    #[test]
    fn a_duplicate_extension_id_is_rejected() {
        let mut host = host();
        host.register(TestExtension::new("apps", "open")).unwrap();
        let error = host
            .register(TestExtension::new("apps", "other"))
            .unwrap_err();
        assert!(matches!(error, HostError::DuplicateExtension(id) if id == "apps"));
    }

    #[test]
    fn an_unsupported_manifest_version_is_rejected() {
        let mut host = host();
        let mut ext = TestExtension::new("apps", "open");
        Arc::get_mut(&mut ext).unwrap().manifest.manifest_version = 99;
        let error = host.register(ext).unwrap_err();
        assert!(matches!(error, HostError::Manifest(_)));
    }

    /// Two extensions that both claim to handle an action, so a test can tell
    /// which one the host actually asked.
    struct Owner(&'static str);

    impl Extension for Owner {
        fn manifest(&self) -> &Manifest {
            // Leaked so the borrow lives as long as the test needs it; a real
            // extension owns its manifest.
            Box::leak(Box::new(Manifest {
                manifest_version: 1,
                id: self.0.into(),
                name: self.0.into(),
                icon: None,
                tint: None,
                commands: vec![],
                preferences: vec![],
                root_items: true,
                services: false,
            }))
        }
        fn perform_action(
            &self,
            item_id: &str,
            action_id: &str,
            values: &FormValues,
        ) -> ActionOutcome {
            let mut submitted: Vec<_> = values.iter().map(|(k, v)| format!("{k}={v}")).collect();
            submitted.sort();
            ActionOutcome::CopyToClipboard(format!(
                "{}:{item_id}:{action_id}:[{}]",
                self.0,
                submitted.join(",")
            ))
        }
    }

    #[test]
    fn a_submitted_forms_values_reach_the_extension() {
        let mut host = host();
        host.register(Arc::new(Owner("alpha"))).unwrap();

        let values = FormValues::from([
            ("name".to_string(), "Nico".to_string()),
            ("city".to_string(), "Berlin".to_string()),
        ]);
        assert_eq!(
            host.perform_action("alpha", "", "save", &values),
            ActionOutcome::CopyToClipboard("alpha::save:[city=Berlin,name=Nico]".into())
        );
    }

    #[test]
    fn an_action_with_no_form_carries_no_values() {
        let mut host = host();
        host.register(Arc::new(Owner("alpha"))).unwrap();
        assert_eq!(
            host.perform_action("alpha", "item", "go", &FormValues::new()),
            ActionOutcome::CopyToClipboard("alpha:item:go:[]".into())
        );
    }

    #[test]
    fn an_action_is_dispatched_to_the_extension_that_owns_the_result() {
        let mut host = host();
        host.register(Arc::new(Owner("alpha"))).unwrap();
        host.register(Arc::new(Owner("beta"))).unwrap();

        assert_eq!(
            host.perform_action("alpha", "item", "go", &FormValues::new()),
            ActionOutcome::CopyToClipboard("alpha:item:go:[]".into())
        );
        assert_eq!(
            host.perform_action("beta", "item", "go", &FormValues::new()),
            ActionOutcome::CopyToClipboard("beta:item:go:[]".into())
        );
    }

    #[test]
    fn an_action_for_a_disabled_extension_fails_rather_than_running() {
        let mut host = host();
        host.register(Arc::new(Owner("alpha"))).unwrap();
        host.set_enabled("alpha", false);
        assert!(matches!(
            host.perform_action("alpha", "item", "go", &FormValues::new()),
            ActionOutcome::Failed(_)
        ));
    }

    #[test]
    fn a_command_of_a_disabled_extension_does_not_resolve() {
        let mut host = host();
        host.register(TestExtension::new("apps", "open")).unwrap();
        host.set_enabled("apps", false);
        assert!(host.resolve_command("apps.open").is_none());
    }

    #[derive(Default)]
    struct Watcher {
        running: AtomicBool,
    }

    impl Service for Watcher {
        fn start(&self) -> ActivationResult {
            self.running.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn stop(&self) {
            self.running.store(false, Ordering::SeqCst);
        }
    }

    struct ServiceExtension {
        manifest: Manifest,
        watcher: Arc<Watcher>,
    }

    impl Extension for ServiceExtension {
        fn manifest(&self) -> &Manifest {
            &self.manifest
        }
        fn services(&self) -> Vec<Arc<dyn Service>> {
            vec![self.watcher.clone()]
        }
    }

    #[test]
    fn services_run_while_enabled_and_stop_when_disabled() {
        let watcher = Arc::new(Watcher::default());
        let extension = Arc::new(ServiceExtension {
            manifest: Manifest {
                manifest_version: 1,
                id: "clip".into(),
                name: "Clipboard".into(),
                icon: None,
                tint: None,
                commands: vec![],
                preferences: vec![],
                root_items: false,
                services: true,
            },
            watcher: watcher.clone(),
        });

        let mut host = host();
        host.register(extension).unwrap();
        assert!(watcher.running.load(Ordering::SeqCst));

        host.set_enabled("clip", false);
        assert!(!watcher.running.load(Ordering::SeqCst));

        host.set_enabled("clip", true);
        assert!(watcher.running.load(Ordering::SeqCst));
    }
}
