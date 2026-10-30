pub mod ai;
pub mod applications;
pub mod clipboard;
pub mod permissions;
pub mod snippets;
pub mod system;
pub mod window_management;

use crate::extension::Manifest;

/// Every built-in extension's manifest as shipped, before the config file has
/// its say: what the config schema and the settings reference describe.
pub fn builtin_manifests() -> Vec<Manifest> {
    use snippets::Kind;
    let shipped_ai = ai::commands::build(&crate::config::Config::default()).commands;
    vec![
        applications::manifest(),
        clipboard::manifest(),
        permissions::manifest(),
        system::manifest(),
        window_management::manifest(),
        ai::manifest(&shipped_ai),
        snippets::manifest(Kind::Snippet),
        snippets::manifest(Kind::Quicklink),
    ]
}

#[cfg(test)]
mod tests {
    use crate::extension::manifest::TINTS;

    #[test]
    fn every_built_in_manifest_validates() {
        for manifest in super::builtin_manifests() {
            if let Err(error) = manifest.validate() {
                panic!("{} does not validate: {error:?}", manifest.id);
            }
        }
    }

    /// A tint the palette does not hold renders as no tint at all, silently, so
    /// the built-ins are checked against it here rather than at load time.
    #[test]
    fn every_built_in_tint_is_one_the_palette_holds() {
        for manifest in super::builtin_manifests() {
            let Some(tint) = &manifest.tint else { continue };
            assert!(
                TINTS.contains(&tint.as_str()),
                "{} declares tint '{tint}', which the palette does not hold",
                manifest.id
            );
        }
    }
}
