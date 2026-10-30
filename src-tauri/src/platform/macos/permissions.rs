use objc2_core_graphics::{CGPreflightListenEventAccess, CGRequestListenEventAccess};

use crate::platform::{Permission, Permissions};

pub struct MacPermissions;

impl Permissions for MacPermissions {
    fn missing(&self) -> Vec<Permission> {
        let mut missing = Vec::new();
        if !super::accessibility_trusted() {
            missing.push(Permission::Accessibility);
        }
        if !CGPreflightListenEventAccess() {
            missing.push(Permission::InputMonitoring);
        }
        missing
    }

    fn request(&self, permission: Permission) {
        match permission {
            Permission::Accessibility => super::prompt_for_accessibility(),
            Permission::InputMonitoring => {
                CGRequestListenEventAccess();
            }
        }
    }

    fn settings_url(&self, permission: Permission) -> Option<&'static str> {
        Some(match permission {
            Permission::Accessibility => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            Permission::InputMonitoring => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
            }
        })
    }
}
