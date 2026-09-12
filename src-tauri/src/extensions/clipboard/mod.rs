//! The `clipboard-history` built-in extension.

mod extension;
mod history;
mod source;
mod watcher;

pub use extension::{preference_declarations, ClipboardExtension, PreferencePolicy, EXTENSION_ID};
pub use history::{Bounds, Content, Entry, History, HistoryError, Kind};
pub use source::{Attribution, ClipboardSource, CrateClipboard};
pub use watcher::{Policy, WatchService, Watcher};

/// Applications excluded until the user says otherwise.
///
/// The spike found Proton Pass setting no privacy marker at all, so there is
/// nothing exact to catch it and this list is the actual defence. An empty
/// default would make the feature a liability out of the box, and there is no
/// preferences interface to fill one in until M7.
///
/// Names are matched against what the platform calls the application, case
/// insensitively. A password manager that runs as a browser extension is not
/// covered: the copying application is then the browser, and excluding a
/// browser would exclude most of what a clipboard history is for.
pub const DEFAULT_EXCLUDED_APPLICATIONS: &[&str] = &[
    "1Password",
    "Bitwarden",
    "Dashlane",
    "Enpass",
    "KeePassXC",
    "Keeper",
    "LastPass",
    "NordPass",
    "Proton Pass",
    "Secrets",
    "Strongbox",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_exclusions_are_sorted_and_unique() {
        let mut sorted = DEFAULT_EXCLUDED_APPLICATIONS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted, DEFAULT_EXCLUDED_APPLICATIONS,
            "kept sorted and unique so adding one is an obvious diff"
        );
    }

    #[test]
    fn the_manager_the_spike_caught_is_excluded_by_default() {
        assert!(DEFAULT_EXCLUDED_APPLICATIONS.contains(&"Proton Pass"));
    }
}
