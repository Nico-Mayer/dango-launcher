//! The `permissions` built-in extension: a warning at the top of root search for
//! each operating-system permission Dango is missing, with the route to grant it.

use std::sync::Arc;

use async_trait::async_trait;

use crate::extension::{ActionOutcome, Extension, FormValues, Manifest};
use crate::extensions::snippets::OpenUrl;
use crate::platform::{Permission, Permissions};
use crate::protocol::Action;
use crate::search::{Candidate, RootProvider, Source};

pub const EXTENSION_ID: &str = "dango.permissions";

pub const ACTION_GRANT: &str = "grant";

pub(crate) fn manifest() -> Manifest {
    Manifest {
        manifest_version: 1,
        id: EXTENSION_ID.into(),
        name: "Permissions".into(),
        icon: None,
        tint: Some("amber".into()),
        commands: vec![],
        preferences: vec![],
        root_items: true,
        services: false,
    }
}

pub struct PermissionsExtension {
    manifest: Manifest,
    permissions: Arc<dyn Permissions>,
    opener: Arc<dyn OpenUrl>,
    provider: Arc<WarningProvider>,
}

impl PermissionsExtension {
    pub fn new(permissions: Arc<dyn Permissions>, opener: Arc<dyn OpenUrl>) -> Self {
        Self {
            manifest: manifest(),
            provider: Arc::new(WarningProvider(permissions.clone())),
            permissions,
            opener,
        }
    }
}

impl Extension for PermissionsExtension {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn root_provider(&self) -> Option<Arc<dyn RootProvider>> {
        Some(self.provider.clone())
    }

    fn perform_action(
        &self,
        item_id: &str,
        action_id: &str,
        _values: &FormValues,
    ) -> ActionOutcome {
        let Some(permission) = from_item_id(item_id).filter(|_| action_id == ACTION_GRANT) else {
            return ActionOutcome::Failed("That action isn't available.".into());
        };
        grant(self.permissions.as_ref(), self.opener.as_ref(), permission)
    }
}

/// Asks for the permission, then opens its settings pane. The order matters:
/// asking is what lists Dango in the pane, so the pane opens with it there.
pub fn grant(
    permissions: &dyn Permissions,
    opener: &dyn OpenUrl,
    permission: Permission,
) -> ActionOutcome {
    permissions.request(permission);
    let Some(url) = permissions.settings_url(permission) else {
        return ActionOutcome::Done;
    };
    match opener.open(url) {
        Ok(()) => ActionOutcome::Done,
        Err(error) => {
            eprintln!("[dango] couldn't open System Settings: {error}");
            ActionOutcome::Failed(format!(
                "Couldn't open System Settings. Open Privacy & Security, {}, and turn on Dango.",
                name(permission)
            ))
        }
    }
}

pub fn name(permission: Permission) -> &'static str {
    match permission {
        Permission::Accessibility => "Accessibility",
        Permission::InputMonitoring => "Input Monitoring",
    }
}

fn item_id(permission: Permission) -> &'static str {
    match permission {
        Permission::Accessibility => "accessibility",
        Permission::InputMonitoring => "input-monitoring",
    }
}

fn from_item_id(id: &str) -> Option<Permission> {
    match id {
        "accessibility" => Some(Permission::Accessibility),
        "input-monitoring" => Some(Permission::InputMonitoring),
        _ => None,
    }
}

fn turns_off(permission: Permission) -> &'static str {
    match permission {
        Permission::Accessibility => {
            "Pasting, snippets, AI paste, and window management don't work without it"
        }
        Permission::InputMonitoring => "Keyword expansion and the hyperkey don't work without it",
    }
}

struct WarningProvider(Arc<dyn Permissions>);

#[async_trait]
impl RootProvider for WarningProvider {
    async fn items(&self, _query: String) -> Vec<Candidate> {
        let mut missing = self.0.missing();
        missing.sort_by_key(|permission| *permission != Permission::Accessibility);
        missing.into_iter().map(warning).collect()
    }
}

fn warning(permission: Permission) -> Candidate {
    Candidate {
        extension_id: EXTENSION_ID.into(),
        id: item_id(permission).into(),
        title: format!("{} is off", name(permission)),
        subtitle: Some(turns_off(permission).into()),
        icon: Some("icon:triangle-alert".into()),
        keywords: vec![
            name(permission).to_lowercase(),
            "permission".into(),
            "privacy".into(),
        ],
        alias: None,
        source: Source::RootItem,
        actions: vec![Action {
            id: ACTION_GRANT.into(),
            title: "Grant access".into(),
            shortcut: None,
        }],
        match_positions: vec![],
        suggested: false,
        pinned: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakePermissions {
        missing: Vec<Permission>,
        requested: Mutex<Vec<Permission>>,
    }

    impl Permissions for FakePermissions {
        fn missing(&self) -> Vec<Permission> {
            self.missing.clone()
        }
        fn request(&self, permission: Permission) {
            self.requested.lock().unwrap().push(permission);
        }
        fn settings_url(&self, _permission: Permission) -> Option<&'static str> {
            Some("settings://pane")
        }
    }

    #[derive(Default)]
    struct FakeOpener(Mutex<Vec<String>>);

    impl OpenUrl for FakeOpener {
        fn open(&self, url: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(url.into());
            Ok(())
        }
    }

    fn build(
        missing: Vec<Permission>,
    ) -> (PermissionsExtension, Arc<FakePermissions>, Arc<FakeOpener>) {
        let permissions = Arc::new(FakePermissions {
            missing,
            ..Default::default()
        });
        let opener = Arc::new(FakeOpener::default());
        (
            PermissionsExtension::new(permissions.clone(), opener.clone()),
            permissions,
            opener,
        )
    }

    async fn items(extension: &PermissionsExtension) -> Vec<Candidate> {
        extension
            .root_provider()
            .unwrap()
            .items(String::new())
            .await
    }

    #[tokio::test]
    async fn nothing_is_listed_when_nothing_is_missing() {
        let (extension, _, _) = build(vec![]);
        assert!(items(&extension).await.is_empty());
    }

    #[tokio::test]
    async fn one_missing_permission_is_one_pinned_warning() {
        let (extension, _, _) = build(vec![Permission::InputMonitoring]);
        let items = items(&extension).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Input Monitoring is off");
        assert!(items[0].pinned);
        assert!(items[0].subtitle.as_deref().unwrap().contains("hyperkey"));
        assert!(items[0].keywords.contains(&"permission".to_string()));
        assert_eq!(items[0].actions[0].title, "Grant access");
    }

    #[tokio::test]
    async fn accessibility_comes_first_when_both_are_missing() {
        let (extension, _, _) = build(vec![Permission::InputMonitoring, Permission::Accessibility]);
        let ids: Vec<String> = items(&extension).await.into_iter().map(|c| c.id).collect();
        assert_eq!(ids, ["accessibility", "input-monitoring"]);
    }

    #[test]
    fn granting_requests_then_opens_the_pane_and_hides() {
        let (extension, permissions, opener) = build(vec![Permission::Accessibility]);
        let outcome = extension.perform_action("accessibility", ACTION_GRANT, &FormValues::new());
        assert_eq!(outcome, ActionOutcome::Done);
        assert_eq!(
            *permissions.requested.lock().unwrap(),
            [Permission::Accessibility]
        );
        assert_eq!(*opener.0.lock().unwrap(), ["settings://pane"]);
    }

    #[test]
    fn an_unknown_item_is_refused() {
        let (extension, permissions, _) = build(vec![]);
        let outcome = extension.perform_action("camera", ACTION_GRANT, &FormValues::new());
        assert!(matches!(outcome, ActionOutcome::Failed(_)));
        assert!(permissions.requested.lock().unwrap().is_empty());
    }
}
