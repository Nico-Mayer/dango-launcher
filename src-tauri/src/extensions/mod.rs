pub mod ai;
pub mod applications;
pub mod clipboard;
pub mod snippets;
pub mod system;
pub mod window_management;

#[cfg(test)]
mod tests {
    use crate::extension::manifest::TINTS;
    use crate::extensions::snippets::Kind;

    /// A tint the palette does not hold renders as no tint at all, silently, so
    /// the built-ins are checked against it here rather than at load time.
    #[test]
    fn every_built_in_tint_is_one_the_palette_holds() {
        let manifests = [
            super::applications::manifest(),
            super::clipboard::manifest(),
            super::system::manifest(),
            super::window_management::manifest(),
            super::ai::manifest(&[]),
            super::snippets::manifest(Kind::Snippet),
            super::snippets::manifest(Kind::Quicklink),
        ];
        for manifest in manifests {
            let Some(tint) = &manifest.tint else { continue };
            assert!(
                TINTS.contains(&tint.as_str()),
                "{} declares tint '{tint}', which the palette does not hold",
                manifest.id
            );
        }
    }
}
