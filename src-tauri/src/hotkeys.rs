//! Turning the command and application hotkeys in the config file into one
//! table of chords, resolving conflicts first-wins and reporting them.
//!
//! The table is pure over the config and the launcher chord. `Registry` holds
//! what is registered with the OS and answers which target a press belongs to.

use std::fmt;
use std::sync::Mutex;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyManager, HotKeyState};

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
    pub chord: HotKey,
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
pub fn bindings(config: &Config, launcher: HotKey) -> Bindings {
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
    launcher: HotKey,
    hyper: global_hotkey::hotkey::Modifiers,
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
        let chord = HotKey::new(Some(parsed.modifiers), parsed.code);

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

/// The launcher shortcut from the config file, or the platform default when it
/// is unset or invalid. Option+Space on macOS and Alt+Space on Windows are the
/// same chord: Option is Alt.
pub fn launcher(config: &Config) -> HotKey {
    if let Some(hotkey) = &config.launcher.hotkey {
        match hotkey.parse_with_hyper(config.hyper_modifiers()) {
            Ok(parsed) => return HotKey::new(Some(parsed.modifiers), parsed.code),
            Err(error) => eprintln!("[dango] invalid launcher hotkey, using the default: {error}"),
        }
    }
    HotKey::new(Some(Modifiers::ALT), Code::Space)
}

/// Taking a chord system-wide and giving it back.
pub trait Grab: Send + Sync {
    fn register(&self, chord: HotKey) -> Result<(), String>;
    fn unregister(&self, chord: HotKey);
}

/// The OS registration. It has to live on the thread whose message loop
/// receives the presses, which is the main thread.
pub struct SystemGrab(GlobalHotKeyManager);

// Safety: created on the main thread and only used there. Every registration
// happens at startup, on a config reload, or at quit, all of which the shell
// runs on the main thread.
unsafe impl Send for SystemGrab {}
unsafe impl Sync for SystemGrab {}

impl SystemGrab {
    pub fn new() -> Result<Self, String> {
        GlobalHotKeyManager::new()
            .map(Self)
            .map_err(|error| error.to_string())
    }
}

impl Grab for SystemGrab {
    fn register(&self, chord: HotKey) -> Result<(), String> {
        self.0.register(chord).map_err(|error| error.to_string())
    }
    fn unregister(&self, chord: HotKey) {
        let _ = self.0.unregister(chord);
    }
}

/// What a press asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pressed {
    Launcher,
    Target(Target),
}

/// The launcher chord and the command bindings currently held. Shared so a
/// reload can rebuild the set while a press looks it up.
pub struct Registry {
    grab: Box<dyn Grab>,
    launcher: Mutex<HotKey>,
    bindings: Mutex<Vec<(HotKey, Target)>>,
}

impl Registry {
    pub fn new(grab: Box<dyn Grab>, launcher: HotKey) -> Self {
        Self {
            grab,
            launcher: Mutex::new(launcher),
            bindings: Mutex::new(Vec::new()),
        }
    }

    pub fn launcher(&self) -> HotKey {
        *self.launcher.lock().unwrap()
    }

    pub fn register_launcher(&self) -> Result<(), String> {
        self.grab.register(self.launcher())
    }

    /// Swaps the launcher chord when it changed.
    pub fn replace_launcher(&self, chord: HotKey) -> Result<(), String> {
        let mut current = self.launcher.lock().unwrap();
        if *current == chord {
            return Ok(());
        }
        self.grab.unregister(*current);
        *current = chord;
        self.grab.register(chord)
    }

    /// Gives back the command chords held now, rebuilds the table first-wins,
    /// registers the survivors, and returns the conflicts to surface: the
    /// collisions plus any the OS refused.
    pub fn apply(&self, config: &Config) -> Vec<String> {
        let mut table = self.bindings.lock().unwrap();
        for (chord, _) in table.drain(..) {
            self.grab.unregister(chord);
        }
        let built = bindings(config, self.launcher());
        let mut conflicts = built.conflicts;
        for binding in built.bindings {
            match self.grab.register(binding.chord) {
                Ok(()) => table.push((binding.chord, binding.target)),
                Err(error) => conflicts.push(format!(
                    "{}: could not register its hotkey, {error}",
                    binding.target
                )),
            }
        }
        conflicts
    }

    /// Only a press counts. The release of the same chord would toggle twice.
    pub fn pressed(&self, id: u32, state: HotKeyState) -> Option<Pressed> {
        if state != HotKeyState::Pressed {
            return None;
        }
        if self.launcher().id() == id {
            return Some(Pressed::Launcher);
        }
        self.bindings
            .lock()
            .unwrap()
            .iter()
            .find(|(chord, _)| chord.id() == id)
            .map(|(_, target)| Pressed::Target(target.clone()))
    }

    /// Hands every chord back, or they stay claimed until the session ends.
    pub fn release_all(&self) {
        self.grab.unregister(self.launcher());
        for (chord, _) in self.bindings.lock().unwrap().drain(..) {
            self.grab.unregister(chord);
        }
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;

    /// Records what is held, and refuses the chords it is told to.
    #[derive(Default)]
    pub struct FakeGrab {
        pub held: std::sync::Arc<Mutex<Vec<HotKey>>>,
        pub refused: Vec<HotKey>,
    }

    impl Grab for FakeGrab {
        fn register(&self, chord: HotKey) -> Result<(), String> {
            if self.refused.contains(&chord) {
                return Err("already in use".into());
            }
            self.held.lock().unwrap().push(chord);
            Ok(())
        }
        fn unregister(&self, chord: HotKey) {
            self.held.lock().unwrap().retain(|held| *held != chord);
        }
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
    use super::testing::FakeGrab;
    use super::*;

    fn config(json: &str) -> Config {
        Config::parse(json).unwrap()
    }

    fn launcher() -> HotKey {
        HotKey::new(Some(Modifiers::ALT), Code::Space)
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

    fn registry(config_json: &str) -> (Registry, std::sync::Arc<Mutex<Vec<HotKey>>>) {
        let grab = FakeGrab::default();
        let held = grab.held.clone();
        let registry = Registry::new(Box::new(grab), launcher());
        registry.register_launcher().unwrap();
        registry.apply(&config(config_json));
        (registry, held)
    }

    #[test]
    fn a_press_on_the_launcher_chord_toggles_the_launcher() {
        let (registry, _) = registry("{}");
        let pressed = registry.pressed(launcher().id(), HotKeyState::Pressed);
        assert_eq!(pressed, Some(Pressed::Launcher));
    }

    #[test]
    fn a_release_does_nothing() {
        let (registry, _) = registry("{}");
        assert_eq!(
            registry.pressed(launcher().id(), HotKeyState::Released),
            None
        );
    }

    #[test]
    fn a_press_on_a_binding_names_its_target() {
        let (registry, _) = registry(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "ctrl+alt+j" } } } } }"#,
        );
        let chord = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyJ);
        assert_eq!(
            registry.pressed(chord.id(), HotKeyState::Pressed),
            Some(Pressed::Target(Target::Command("x.a".into())))
        );
    }

    #[test]
    fn a_reload_gives_back_chords_no_longer_bound() {
        let (registry, held) = registry(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "ctrl+alt+j" } } } } }"#,
        );
        assert_eq!(held.lock().unwrap().len(), 2);
        registry.apply(&config("{}"));
        assert_eq!(*held.lock().unwrap(), vec![launcher()]);
    }

    #[test]
    fn a_refused_chord_is_reported_and_not_held() {
        let refused = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyJ);
        let grab = FakeGrab {
            refused: vec![refused],
            ..Default::default()
        };
        let registry = Registry::new(Box::new(grab), launcher());
        let conflicts = registry.apply(&config(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "ctrl+alt+j" } } } } }"#,
        ));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(registry.pressed(refused.id(), HotKeyState::Pressed), None);
    }

    #[test]
    fn a_new_launcher_chord_replaces_the_old_one() {
        let (registry, held) = registry("{}");
        let chord = HotKey::new(Some(Modifiers::CONTROL), Code::Space);
        registry.replace_launcher(chord).unwrap();
        assert_eq!(*held.lock().unwrap(), vec![chord]);
        assert_eq!(
            registry.pressed(chord.id(), HotKeyState::Pressed),
            Some(Pressed::Launcher)
        );
    }

    #[test]
    fn quitting_releases_every_chord() {
        let (registry, held) = registry(
            r#"{ "extensions": { "x": { "commands": { "a": { "hotkey": "ctrl+alt+j" } } } } }"#,
        );
        registry.release_all();
        assert!(held.lock().unwrap().is_empty());
    }
}
