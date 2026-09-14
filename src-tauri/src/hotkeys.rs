//! Turning the command and application hotkeys in the config file into one
//! table of chords, resolving conflicts first-wins and reporting them.
//!
//! Pure over the config and the launcher chord: registering the survivors and
//! dispatching a press live in `lib.rs`, which owns the shortcut plugin and the
//! invocation path.

use std::fmt;

use tauri_plugin_global_shortcut::Shortcut;

use crate::config::Config;
use crate::extensions::applications::IndexedApp;

/// What a chord runs: a command by its qualified `extension.command` id, or an
/// application by the name root search shows for it, resolved at the press.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Command(String),
    Application(String),
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Command(id) => f.write_str(id),
            Target::Application(name) => write!(f, "application \"{name}\""),
        }
    }
}

pub struct Binding {
    pub chord: Shortcut,
    pub target: Target,
}

/// The bindings the config asks for, and the conflicts that kept some from
/// applying. Conflicts are ready-to-show messages naming the commands involved.
pub struct Bindings {
    pub bindings: Vec<Binding>,
    pub conflicts: Vec<String>,
}

/// Builds the binding table. Iterating the config in its stable key order,
/// commands before applications within an extension, makes first-wins
/// deterministic: the first target to claim a chord keeps it, and later
/// claims, or a claim on the launcher chord, are reported rather than applied.
pub fn bindings(config: &Config, launcher: Shortcut) -> Bindings {
    let mut table = Table {
        launcher,
        hyper: config.hyper_modifiers(),
        bindings: Vec::new(),
        conflicts: Vec::new(),
    };

    for (extension_id, extension) in &config.extensions {
        for (command_id, command) in &extension.commands {
            if let Some(hotkey) = &command.hotkey {
                table.claim(
                    Target::Command(format!("{extension_id}.{command_id}")),
                    hotkey,
                );
            }
        }
        for (key, app) in &extension.apps {
            let Some(hotkey) = &app.hotkey else {
                continue;
            };
            // An entry naming only the other platform is not for this machine.
            if let Some(name) = app.name_for_platform(key) {
                table.claim(Target::Application(name.to_string()), hotkey);
            }
        }
    }

    Bindings {
        bindings: table.bindings,
        conflicts: table.conflicts,
    }
}

struct Table {
    launcher: Shortcut,
    hyper: tauri_plugin_global_shortcut::Modifiers,
    bindings: Vec<Binding>,
    conflicts: Vec<String>,
}

impl Table {
    fn claim(&mut self, target: Target, hotkey: &crate::config::Hotkey) {
        let parsed = match hotkey.parse_with_hyper(self.hyper) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.conflicts
                    .push(format!("{target}: invalid hotkey, {error}"));
                return;
            }
        };
        let chord = Shortcut::new(Some(parsed.modifiers), parsed.code);

        if chord == self.launcher {
            self.conflicts.push(format!(
                "{target} is bound to the launcher hotkey; the launcher keeps it"
            ));
            return;
        }
        if let Some(existing) = self.bindings.iter().find(|b| b.chord == chord) {
            self.conflicts.push(format!(
                "{target} and {} are bound to the same hotkey; {} keeps it",
                existing.target, existing.target
            ));
            return;
        }
        self.bindings.push(Binding { chord, target });
    }
}

/// What a press on an application binding should do, given the applications
/// whose shown name matched. Two matches launch the first and say so, because
/// a silent guess is worse than a launch the user can then disambiguate.
pub enum Resolution {
    Launch {
        app: IndexedApp,
        ambiguity: Option<String>,
    },
    Missing(String),
}

pub fn resolve_application(name: &str, matches: Vec<IndexedApp>) -> Resolution {
    let mut matches = matches;
    let Some(app) = matches.first().cloned() else {
        return Resolution::Missing(format!(
            "application \"{name}\" is bound to a hotkey but is not installed"
        ));
    };
    let ambiguity = (matches.len() > 1).then(|| {
        let others: Vec<String> = matches.drain(1..).map(|a| a.id).collect();
        format!(
            "application \"{name}\" matches more than one installed application; launching {} rather than {}",
            app.id,
            others.join(", ")
        )
    });
    Resolution::Launch { app, ambiguity }
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
        let result = bindings(&c, launcher());
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
        let result = bindings(&c, launcher());
        assert_eq!(result.bindings.len(), 1);
        assert_eq!(
            result.bindings[0].target,
            Target::Command("x.alpha".into()),
            "first wins"
        );
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("x.beta"));
        assert!(result.conflicts[0].contains("x.alpha"));
    }

    #[test]
    fn a_command_on_the_launcher_chord_is_reported() {
        let c = config(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "alt+space" } } } } }"#,
        );
        let result = bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("launcher"));
    }

    #[test]
    fn an_unparseable_hotkey_is_reported() {
        let c = config(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "ctrl+nope" } } } } }"#,
        );
        let result = bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("x.a"));
    }

    #[test]
    fn an_application_is_bound_by_its_shown_name() {
        let c = config(
            r#"{ "extensions": { "dango.applications": { "apps": {
                "Safari": { "hotkey": "ctrl+alt+b" },
                "code": { "name": "Visual Studio Code", "hotkey": "ctrl+alt+c" }
            } } } }"#,
        );
        let result = bindings(&c, launcher());
        assert!(result.conflicts.is_empty());
        let targets: Vec<_> = result.bindings.iter().map(|b| b.target.clone()).collect();
        assert_eq!(
            targets,
            [
                Target::Application("Safari".into()),
                Target::Application("Visual Studio Code".into())
            ]
        );
    }

    #[test]
    fn an_application_and_a_command_on_one_chord_are_reported_by_name() {
        let c = config(
            r#"{ "extensions": {
                "dango.applications": { "apps": { "Safari": { "hotkey": "ctrl+alt+j" } } },
                "x": { "commands": { "alpha": { "hotkey": "ctrl+alt+j" } } }
            } }"#,
        );
        let result = bindings(&c, launcher());
        assert_eq!(result.bindings.len(), 1);
        assert_eq!(
            result.bindings[0].target,
            Target::Application("Safari".into()),
            "dango.applications sorts before x, so the application claimed first"
        );
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.conflicts[0].contains("x.alpha"));
        assert!(result.conflicts[0].contains("application \"Safari\""));
    }

    #[test]
    fn an_entry_naming_only_the_other_platform_is_skipped() {
        #[cfg(target_os = "macos")]
        let other = "windows";
        #[cfg(not(target_os = "macos"))]
        let other = "macos";
        let c = config(&format!(
            r#"{{ "extensions": {{ "dango.applications": {{ "apps": {{
                "elsewhere": {{ "name": {{ "{other}": "Only There" }}, "hotkey": "ctrl+alt+e" }}
            }} }} }} }}"#
        ));
        let result = bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert!(result.conflicts.is_empty());
    }

    fn app(id: &str, name: &str) -> IndexedApp {
        IndexedApp {
            id: id.into(),
            name: name.into(),
            target: None,
        }
    }

    #[test]
    fn a_single_match_launches_quietly() {
        match resolve_application("Safari", vec![app("/Applications/Safari.app", "Safari")]) {
            Resolution::Launch { app, ambiguity } => {
                assert_eq!(app.id, "/Applications/Safari.app");
                assert!(ambiguity.is_none());
            }
            Resolution::Missing(_) => panic!("one match is a launch"),
        }
    }

    #[test]
    fn two_matches_launch_the_first_and_say_so() {
        match resolve_application("Code", vec![app("a", "Code"), app("b", "Code")]) {
            Resolution::Launch { app, ambiguity } => {
                assert_eq!(app.id, "a");
                let note = ambiguity.expect("ambiguity is reported");
                assert!(note.contains("\"Code\""));
                assert!(note.contains('b'));
            }
            Resolution::Missing(_) => panic!("two matches still launch"),
        }
    }

    #[test]
    fn no_match_is_reported_with_the_name() {
        match resolve_application("Nope", vec![]) {
            Resolution::Missing(message) => {
                assert!(message.contains("\"Nope\""));
                assert!(message.contains("not installed"));
            }
            Resolution::Launch { .. } => panic!("nothing to launch"),
        }
    }

    #[test]
    fn commands_without_a_hotkey_are_ignored() {
        let c = config(r#"{ "extensions": { "x": { "commands": { "a": { "alias": "aa" } } } } }"#);
        let result = bindings(&c, launcher());
        assert!(result.bindings.is_empty());
        assert!(result.conflicts.is_empty());
    }
}
