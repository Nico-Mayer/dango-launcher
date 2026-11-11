//! One table per built-in extension, named by its id without `dango.`. Each
//! table carries the two keys Dango reads for every extension, `enabled` and
//! `commands`, beside the extension's own typed settings.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{AppSettings, CommandSettings, Hotkey, Output, ProviderSettings, Thinking};
use crate::extensions::clipboard::{Bounds, DEFAULT_EXCLUDED_APPLICATIONS, MEGABYTE};

/// What Dango reads from any table, whatever else the extension keeps there.
pub trait ExtensionTable {
    fn enabled(&self) -> Option<bool>;
    fn commands(&self) -> Vec<(&str, CommandBinding<'_>)>;
    fn command(&self, id: &str) -> Option<CommandBinding<'_>>;
}

/// The per-command keys every extension shares.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandBinding<'a> {
    pub enabled: Option<bool>,
    pub alias: Option<&'a str>,
    pub hotkey: Option<&'a Hotkey>,
}

trait Binds {
    fn binding(&self) -> CommandBinding<'_>;
}

impl Binds for CommandSettings {
    fn binding(&self) -> CommandBinding<'_> {
        CommandBinding {
            enabled: self.enabled,
            alias: self.alias.as_deref(),
            hotkey: self.hotkey.as_ref(),
        }
    }
}

impl Binds for AiCommandSettings {
    fn binding(&self) -> CommandBinding<'_> {
        CommandBinding {
            enabled: self.enabled,
            alias: self.alias.as_deref(),
            hotkey: self.hotkey.as_ref(),
        }
    }
}

macro_rules! extension_table {
    ($($table:ty),+) => {$(
        impl ExtensionTable for $table {
            fn enabled(&self) -> Option<bool> {
                self.enabled
            }

            fn commands(&self) -> Vec<(&str, CommandBinding<'_>)> {
                self.commands
                    .iter()
                    .map(|(id, command)| (id.as_str(), command.binding()))
                    .collect()
            }

            fn command(&self, id: &str) -> Option<CommandBinding<'_>> {
                self.commands.get(id).map(Binds::binding)
            }
        }
    )+};
}

extension_table!(
    Table,
    ClipboardTable,
    DisplayTable,
    QuicklinksTable,
    AiTable,
    ApplicationsTable
);

/// A table for an extension with no settings of its own.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Table {
    /// Whether the extension runs. Absent means enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Per-command settings, keyed by command id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
}

/// The table of an extension that is not built in: what Dango reads for it,
/// and every other key as the extension's own preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionSettings {
    #[serde(flatten)]
    pub table: Table,
    /// The extension's own settings, handed to it as they are.
    #[serde(flatten)]
    pub preferences: BTreeMap<String, serde_json::Value>,
}

impl ExtensionTable for ExtensionSettings {
    fn enabled(&self) -> Option<bool> {
        self.table.enabled()
    }

    fn commands(&self) -> Vec<(&str, CommandBinding<'_>)> {
        self.table.commands()
    }

    fn command(&self, id: &str) -> Option<CommandBinding<'_>> {
        self.table.command(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "kebab-case")]
pub struct ClipboardTable {
    /// Whether the extension runs. Absent means enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Per-command settings, keyed by command id.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
    /// How many entries the history keeps.
    pub entries: u32,
    /// The largest single entry kept, in megabytes.
    pub entry_megabytes: u32,
    /// The most the whole history may use, in megabytes.
    pub total_megabytes: u32,
    /// Applications whose copies are never recorded, matched ignoring case.
    pub excluded_applications: Vec<String>,
    /// What Enter does to an entry: paste it into the previous app, or only
    /// copy it.
    pub primary_action: PrimaryAction,
}

impl Default for ClipboardTable {
    fn default() -> Self {
        let bounds = Bounds::default();
        Self {
            enabled: None,
            commands: BTreeMap::new(),
            entries: bounds.entries as u32,
            entry_megabytes: (bounds.entry_bytes / MEGABYTE) as u32,
            total_megabytes: (bounds.total_bytes / MEGABYTE) as u32,
            excluded_applications: DEFAULT_EXCLUDED_APPLICATIONS
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            primary_action: PrimaryAction::default(),
        }
    }
}

/// What Enter does to a clipboard history entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PrimaryAction {
    #[default]
    Paste,
    Copy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "kebab-case")]
pub struct DisplayTable {
    /// Whether the extension runs. Absent means enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Per-command settings, keyed by command id.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
    /// Which monitors the brightness commands change. "all" changes every
    /// external monitor, "pointer" only the one under the mouse pointer.
    /// Default "all".
    pub monitors: BrightnessTarget,
}

/// Which monitors a brightness command changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BrightnessTarget {
    #[default]
    All,
    Pointer,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "kebab-case")]
pub struct QuicklinksTable {
    /// Whether the extension runs. Absent means enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Per-command settings, keyed by command id.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
    /// Show each quicklink with its site's icon. Off stops Dango fetching from
    /// those sites.
    pub favicons: bool,
}

impl Default for QuicklinksTable {
    fn default() -> Self {
        Self {
            enabled: None,
            commands: BTreeMap::new(),
            favicons: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "kebab-case")]
pub struct AiTable {
    /// Whether the extension runs. Absent means enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Per-command settings, keyed by command id. A command id the extension
    /// does not ship defines a new command.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, AiCommandSettings>,
    /// Model services, keyed by the name commands refer to them by. That name
    /// is also the key its API key is filed under in auth.toml.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub providers: BTreeMap<String, ProviderSettings>,
    /// The provider a command uses when it names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// How hard the model thinks when a command does not say.
    pub thinking: Thinking,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct AiCommandSettings {
    /// A global hotkey that runs this command from anywhere. The command shows
    /// its view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<Hotkey>,
    /// A short alias that matches this command in root search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Whether the command is offered. Absent means yes; false removes a
    /// command the extension ships.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The title root search shows. Needed for a command the file defines
    /// from nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What the model is asked, as a template. Placeholders: {{ selection }},
    /// {{ clipboard }}, and any other name, which the launcher asks for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The provider this command uses, by its name under providers. Absent
    /// falls back to the extension's default provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// The model this command uses. Absent falls back to the provider's model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// How hard the model thinks before answering. Absent falls back to the
    /// extension's default level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    /// What becomes of the answer: shown in the launcher, pasted into the
    /// previous app, or copied. Default view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Output>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "kebab-case")]
pub struct ApplicationsTable {
    /// Whether the extension runs. Absent means enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Per-command settings, keyed by command id.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, CommandSettings>,
    /// Applications bound to global hotkeys, keyed by the name Dango shows for
    /// the application in root search.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub hotkeys: BTreeMap<String, AppSettings>,
}

/// The table a built-in extension is configured under: its id without the
/// `dango.` prefix. `None` for an extension that is not built in.
pub fn table_name(extension_id: &str) -> Option<&str> {
    extension_id.strip_prefix("dango.")
}
