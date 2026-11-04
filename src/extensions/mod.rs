pub mod ai;
pub mod applications;
pub mod calculator;
pub mod clipboard;
pub mod display;
pub mod games;
pub mod permissions;
pub mod refresh;
pub mod snippets;
pub mod system;
pub mod updates;
pub mod window_management;

use crate::extension::Manifest;

/// Every built-in extension's manifest as shipped, before the config file has
/// its say: what the config schema and the settings reference describe.
pub fn builtin_manifests() -> Vec<Manifest> {
    use snippets::Kind;
    let shipped_ai = ai::commands::build(&crate::config::Config::default()).commands;
    vec![
        applications::manifest(),
        calculator::manifest(),
        clipboard::manifest(),
        display::manifest(),
        games::manifest(),
        permissions::manifest(),
        system::manifest(),
        updates::manifest(),
        window_management::manifest(),
        ai::manifest(&shipped_ai),
        snippets::manifest(Kind::Snippet),
        snippets::manifest(Kind::Quicklink),
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_built_in_manifest_validates() {
        for manifest in super::builtin_manifests() {
            if let Err(error) = manifest.validate() {
                panic!("{} does not validate: {error:?}", manifest.id);
            }
        }
    }
}
