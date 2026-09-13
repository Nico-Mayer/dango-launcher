//! Turning the command hotkeys in the config file into a table of chords to
//! invoke, resolving conflicts first-wins and reporting them.
//!
//! Pure over the config and the launcher chord: registering the survivors and
//! dispatching a press live in `lib.rs`, which owns the shortcut plugin and the
//! invocation path.

use tauri_plugin_global_shortcut::Shortcut;

use crate::config::Config;

/// One command bound to a chord. The id is qualified, `extension.command`.
pub struct Binding {
    pub chord: Shortcut,
    pub command_id: String,
}

/// The bindings the config asks for, and the conflicts that kept some from
/// applying. Conflicts are ready-to-show messages naming the commands involved.
pub struct Bindings {
    pub bindings: Vec<Binding>,
    pub conflicts: Vec<String>,
}

/// Builds the command-binding table. Iterating the config in its stable key
/// order makes first-wins deterministic: the first command to claim a chord
/// keeps it, and later claims, or a claim on the launcher chord, are reported
/// rather than applied.
pub fn command_bindings(config: &Config, launcher: Shortcut) -> Bindings {
    let mut bindings: Vec<Binding> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();
    let hyper = config.hyper_modifiers();

    for (extension_id, extension) in &config.extensions {
        for (command_id, command) in &extension.commands {
            let Some(hotkey) = &command.hotkey else {
                continue;
            };
            let qualified = format!("{extension_id}.{command_id}");
            let parsed = match hotkey.parse_with_hyper(hyper) {
                Ok(parsed) => parsed,
                Err(error) => {
                    conflicts.push(format!("{qualified}: invalid hotkey, {error}"));
                    continue;
                }
            };
            let chord = Shortcut::new(Some(parsed.modifiers), parsed.code);

            if chord == launcher {
                conflicts.push(format!(
                    "{qualified} is bound to the launcher hotkey; the launcher keeps it"
                ));
                continue;
            }
            if let Some(existing) = bindings.iter().find(|b| b.chord == chord) {
                conflicts.push(format!(
                    "{qualified} and {} are bound to the same hotkey; {} keeps it",
                    existing.command_id, existing.command_id
                ));
                continue;
            }
            bindings.push(Binding {
                chord,
                command_id: qualified,
            });
        }
    }

    Bindings {
        bindings,
        conflicts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    fn config(json: &str) -> Config {
        Config::parse(json).unwrap()
    }

    fn launcher() -> Shortcut {
        Shortcut::new(Some(Modifiers::ALT), Code::Space)
    }

    #[test]
    fn a_clean_set_binds_every_command() {
        let c = config(
            r#"{ "extensions": {
                "dango.window-management": { "commands": {
                    "left-half": { "hotkey": "ctrl+alt+left" },
                    "right-half": { "hotkey": "ctrl+alt+right" }
                } }
            } }"#,
        );
        let result = command_bindings(&c, launcher());
        assert_eq!(result.bindings.len(), 2);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn two_commands_on_one_chord_first_wins_and_is_reported() {
        let c = config(
            r#"{ "extensions": { "x": { "commands": {
                "alpha": { "hotkey": "ctrl+alt+j" },
                "beta": { "hotkey": "ctrl+alt+j" }
            } } } }"#,
        );
        let result = command_bindings(&c, launcher());
        assert_eq!(result.bindings.len(), 1);
        assert_eq!(result.bindings[0].command_id, "x.alpha", "first wins");
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("x.beta"));
        assert!(result.conflicts[0].contains("x.alpha"));
    }

    #[test]
    fn a_command_on_the_launcher_chord_is_reported() {
        let c = config(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "alt+space" } } } } }"#,
        );
        let result = command_bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("launcher"));
    }

    #[test]
    fn an_unparseable_hotkey_is_reported() {
        let c = config(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "ctrl+nope" } } } } }"#,
        );
        let result = command_bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("x.a"));
    }

    #[test]
    fn commands_without_a_hotkey_are_ignored() {
        let c = config(r#"{ "extensions": { "x": { "commands": { "a": { "alias": "aa" } } } } }"#);
        let result = command_bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert!(result.conflicts.is_empty());
    }
}
