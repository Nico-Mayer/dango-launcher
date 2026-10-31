use crate::platform::{Permission, Permissions};

pub struct WindowsPermissions;

impl Permissions for WindowsPermissions {
    fn missing(&self) -> Vec<Permission> {
        Vec::new()
    }

    fn request(&self, _permission: Permission) {}

    fn settings_url(&self, _permission: Permission) -> Option<&'static str> {
        None
    }
}
