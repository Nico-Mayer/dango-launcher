pub mod manifest;
pub mod registry;

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

pub use manifest::{CommandDecl, InvocationMode, Manifest, ManifestError};
pub use registry::{Collision, Host, RegisteredCommand, Registry};

pub type ActivationResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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

impl EnabledStore for crate::store::Store {
    fn is_enabled(&self, extension_id: &str) -> bool {
        self.extension_enabled(extension_id).unwrap_or(true)
    }
    fn set_enabled(&self, extension_id: &str, enabled: bool) {
        if let Err(error) = self.set_extension_enabled(extension_id, enabled) {
            eprintln!("[dango] could not persist enabled state for {extension_id}: {error}");
        }
    }
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
