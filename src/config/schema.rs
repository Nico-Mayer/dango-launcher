//! The JSON Schema for each user file, generated from the structs that read
//! it and from the built-in manifests, so an editor's completions and the
//! settings reference cannot drift from what the code accepts.

use std::fmt::Write as _;
use std::path::Path;

use serde_json::{json, Map, Value};

use super::files::UserFile;
use super::hotkey::{stands_alone, KEY_TOKENS, MODIFIER_TOKENS};
use super::{table_name, Config};
use crate::browser::Browser;
use crate::extension::Manifest;
use crate::extensions::snippets::{QuicklinkEntry, SnippetEntry};

/// The one extension whose commands the file may also define, so its
/// `commands` stays open to ids it does not ship.
const AI_ID: &str = "dango.ai";

pub fn generate(file: UserFile, manifests: &[Manifest]) -> Value {
    let mut schema = match file {
        UserFile::Config => config_schema(manifests),
        UserFile::Snippets => records_schema(
            schemars::schema_for!(SnippetEntry).to_value(),
            "Dango snippets",
            "Snippets, each a table keyed by its id. Dango adds, edits, and removes \
             them from the launcher and keeps your comments and order.",
        ),
        UserFile::Quicklinks => {
            let mut schema = records_schema(
                schemars::schema_for!(QuicklinkEntry).to_value(),
                "Dango quicklinks",
                "Quicklinks, each a table keyed by its id. Dango adds, edits, and removes \
                 them from the launcher and keeps your comments and order.",
            );
            let browsers: Vec<&str> = Browser::ALL.iter().map(|b| b.id()).collect();
            schema["$defs"]["QuicklinkEntry"]["properties"]["browser"]["enum"] = json!(browsers);
            schema
        }
        UserFile::Auth => auth_schema(),
    };
    drop_unused_defs(&mut schema);
    tidy(&mut schema);
    schema
}

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

fn version_property() -> Value {
    json!({
        "type": "integer",
        "minimum": 0,
        "description": "The file's schema version. Leave it at 1."
    })
}

/// A file whose top level holds `version` and records keyed by id, each
/// described by `entry`.
fn records_schema(mut entry: Value, title: &str, description: &str) -> Value {
    strip_null(&mut entry);
    close_objects(&mut entry);
    let name = entry["title"].as_str().unwrap_or("Entry").to_string();
    let entry_description = entry["description"].clone();
    if let Some(fields) = entry.as_object_mut() {
        fields.remove("$schema");
        fields.remove("title");
    }
    let mut defs = Map::new();
    defs.insert(name.clone(), entry);
    json!({
        "$schema": DRAFT,
        "$defs": defs,
        "title": title,
        "description": description,
        "type": "object",
        "properties": { "version": version_property() },
        "additionalProperties": {
            "$ref": format!("#/$defs/{name}"),
            "description": entry_description
        }
    })
}

fn auth_schema() -> Value {
    json!({
        "$schema": DRAFT,
        "title": "Dango keys",
        "description": "API keys for hosted AI providers, keyed by the provider's name in \
            config.toml. Dango only reads this file. Keep it out of git.",
        "type": "object",
        "properties": { "version": version_property() },
        "additionalProperties": {
            "type": "string",
            "description": "The provider's API key."
        }
    })
}

fn config_schema(manifests: &[Manifest]) -> Value {
    let mut schema = schemars::schema_for!(Config).to_value();
    strip_null(&mut schema);
    close_objects(&mut schema);
    describe_root(&mut schema);
    constrain_hotkeys(&mut schema);
    constrain_hyperkey(&mut schema);
    inline_tables(&mut schema, manifests);
    schema
}

pub fn render(schema: &Value) -> String {
    let mut text = serde_json::to_string_pretty(schema).unwrap_or_default();
    text.push('\n');
    text
}

/// Writes every user file's schema under `dir`, returning one message per
/// schema that could not be written.
pub fn write_all(dir: &Path, manifests: &[Manifest]) -> Vec<String> {
    UserFile::ALL
        .into_iter()
        .filter_map(|file| write(dir, file, manifests).err())
        .collect()
}

/// Writes one schema unless the copy there already matches. Returns whether a
/// write happened.
pub fn write(dir: &Path, file: UserFile, manifests: &[Manifest]) -> Result<bool, String> {
    let path = file.schema_path_in(dir);
    let text = render(&generate(file, manifests));
    if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
        return Ok(false);
    }
    path.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| std::fs::write(&path, &text))
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(true)
}

fn defs(schema: &mut Value) -> &mut Map<String, Value> {
    schema["$defs"]
        .as_object_mut()
        .expect("schemars emits $defs")
}

/// Config never writes `null`, so an optional setting is just one that may be
/// absent. Drops the `null` schemars adds for every `Option`.
fn strip_null(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if let Some(Value::Array(types)) = map.get_mut("type") {
                types.retain(|t| t != "null");
                if types.len() == 1 {
                    let only = types.remove(0);
                    map.insert("type".into(), only);
                }
            }
            if let Some(Value::Array(branches)) = map.get("anyOf").cloned() {
                let kept: Vec<Value> = branches
                    .into_iter()
                    .filter(|b| b.get("type") != Some(&json!("null")))
                    .collect();
                if kept.len() == 1 {
                    map.remove("anyOf");
                    if let Value::Object(fields) = kept.into_iter().next().unwrap() {
                        for (key, value) in fields {
                            map.entry(key).or_insert(value);
                        }
                    }
                } else {
                    map.insert("anyOf".into(), Value::Array(kept));
                }
            }
            map.values_mut().for_each(strip_null);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_null),
        _ => {}
    }
}

/// Every object that names its properties rejects any other. Maps keyed by
/// the user's own names carry an item schema instead and are left open.
fn close_objects(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if map.contains_key("properties")
                && !map
                    .get("additionalProperties")
                    .is_some_and(Value::is_object)
            {
                map.insert("additionalProperties".into(), Value::Bool(false));
            }
            map.values_mut().for_each(close_objects);
        }
        Value::Array(items) => items.iter_mut().for_each(close_objects),
        _ => {}
    }
}

/// Doc comments arrive with their line breaks and number defaults as floats;
/// neither reads well in a hover or a table.
fn tidy(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if let Some(Value::String(text)) = map.get_mut("description") {
                *text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            }
            if let Some(Value::Number(number)) = map.get("default") {
                if let Some(whole) = number.as_f64().filter(|f| f.fract() == 0.0) {
                    map.insert("default".into(), json!(whole as i64));
                }
            }
            map.values_mut().for_each(tidy);
        }
        Value::Array(items) => items.iter_mut().for_each(tidy),
        _ => {}
    }
}

fn describe_root(schema: &mut Value) {
    schema["title"] = json!("Dango configuration");
    schema["description"] = json!(
        "Every setting is optional; anything left out uses its built-in default. \
         Dango keeps keys it does not know, but this schema flags them."
    );
}

fn hotkey_pattern() -> String {
    let modifiers = MODIFIER_TOKENS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join("|");
    let keys = KEY_TOKENS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join("|");
    let lone = KEY_TOKENS
        .iter()
        .filter(|(_, code)| stands_alone(*code))
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join("|");
    format!("^(?:(?:(?:{modifiers})\\+)+(?:{keys}|[a-z0-9])|(?:{lone}))$")
}

/// The chord grammar as a pattern on both shapes a hotkey can take. The
/// per-platform object is inlined here rather than patterned on the shared
/// `PerPlatform` def, which application names use too.
fn constrain_hotkeys(schema: &mut Value) {
    let pattern = hotkey_pattern();
    let chord = |description: &str| json!({ "type": "string", "pattern": pattern, "description": description });
    let hotkey = &mut defs(schema)["Hotkey"];
    hotkey["anyOf"] = json!([
        chord("A chord: modifiers joined with +, then one key, such as mod+shift+k."),
        {
            "type": "object",
            "additionalProperties": false,
            "description": "A chord per platform. A missing side means no binding there.",
            "properties": {
                "macos": chord("The chord on macOS."),
                "windows": chord("The chord on Windows.")
            }
        }
    ]);
}

fn key_names() -> Vec<String> {
    KEY_TOKENS
        .iter()
        .map(|(name, _)| name.to_string())
        .chain(('a'..='z').map(String::from))
        .chain(('0'..='9').map(String::from))
        .collect()
}

fn constrain_hyperkey(schema: &mut Value) {
    let hyperkey = &mut defs(schema)["Hyperkey"]["properties"];
    hyperkey["tap"]["enum"] = json!(key_names());
}

/// Each built-in table is written out in place, so its `commands` can name
/// the ids that extension ships. The AI table also accepts ids it does not
/// ship, because the file defines new AI commands there.
fn inline_tables(schema: &mut Value, manifests: &[Manifest]) {
    for manifest in manifests {
        let Some(name) = table_name(&manifest.id) else {
            continue;
        };
        let property = schema["properties"][name].clone();
        let mut table = resolve(schema, &property).0.clone();
        table["description"] = property["description"].clone();
        let commands = &mut table["properties"]["commands"];
        let item = commands["additionalProperties"].clone();
        let mut properties = Map::new();
        for command in &manifest.commands {
            properties.insert(
                command.id.clone(),
                json!({
                    "$ref": item["$ref"],
                    "description": format!("Settings for the {} command.", command.id)
                }),
            );
        }
        commands["properties"] = Value::Object(properties);
        if manifest.id != AI_ID {
            commands["additionalProperties"] = json!(false);
        }
        schema["properties"][name] = table;
    }
}

fn drop_unused_defs(schema: &mut Value) {
    let Some(defs_map) = schema.get("$defs").and_then(Value::as_object) else {
        return;
    };
    let names: Vec<String> = defs_map.keys().cloned().collect();
    for name in names {
        let reference = format!("\"#/$defs/{name}\"");
        if !schema.to_string().contains(&reference) {
            defs(schema).remove(&name);
        }
    }
}

/// The settings reference as Markdown: a part per user file, one table per
/// object, every property with its type, default, allowed values, and
/// description. Shared shapes from `$defs` are rendered once and linked to.
pub fn reference(manifests: &[Manifest]) -> String {
    let mut out = String::from(
        "# Dango configuration\n\nEvery file in the config directory and every setting in it. \
         Generated from the code; do not edit by hand.\n",
    );
    for file in UserFile::ALL {
        let schema = generate(file, manifests);
        let _ = writeln!(out, "\n## {}\n", file.file_name());
        if let Some(description) = schema["description"].as_str() {
            let _ = writeln!(out, "{description}");
        }
        let mut writer = Reference {
            schema: &schema,
            out: &mut out,
            pending: Vec::new(),
            rendered: Vec::new(),
        };
        writer.section("Top level", "", &schema);
        while let Some(name) = writer.pending.first().cloned() {
            writer.pending.remove(0);
            writer.rendered.push(name.clone());
            let def = schema["$defs"][&name].clone();
            if name == "Hotkey" {
                writer.hotkey_section(&def_title(&name), &def);
            } else {
                writer.section(&def_title(&name), "", &def);
            }
        }
    }
    out
}

struct Reference<'a> {
    schema: &'a Value,
    out: &'a mut String,
    pending: Vec<String>,
    rendered: Vec<String>,
}

impl Reference<'_> {
    fn section(&mut self, title: &str, path: &str, node: &Value) {
        if let Some(branches) = node["anyOf"].as_array() {
            self.shapes_section(title, node, branches);
            return;
        }
        let Some(properties) = node["properties"].as_object() else {
            return;
        };
        let _ = writeln!(self.out, "\n### {title}\n");
        if let Some(description) = node["description"].as_str().filter(|_| !path.is_empty()) {
            let _ = writeln!(self.out, "{description}\n");
        }
        let _ = writeln!(
            self.out,
            "| Setting | Type | Default | Values | Description |"
        );
        let _ = writeln!(self.out, "| --- | --- | --- | --- | --- |");
        let schema = self.schema;
        let mut children = Vec::new();
        for (name, property) in properties {
            let (resolved, def) = resolve(schema, property);
            let nothing_to_set = def.is_none()
                && resolved["properties"]
                    .as_object()
                    .is_some_and(Map::is_empty);
            if nothing_to_set {
                continue;
            }
            let description = property["description"]
                .as_str()
                .or(resolved["description"].as_str())
                .unwrap_or("");
            let _ = writeln!(
                self.out,
                "| `{name}` | {} | {} | {} | {} |",
                type_name(schema, property),
                default_of(if property.get("default").is_some() {
                    property
                } else {
                    resolved
                }),
                values_of(resolved),
                description
            );
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if let Some(def) = def {
                self.enqueue(def);
            } else if resolved.get("properties").is_some() {
                children.push((child_path, resolved.clone()));
            } else if let Some(item) = resolved
                .get("additionalProperties")
                .filter(|v| v.is_object())
            {
                let (item, def) = resolve(schema, item);
                if let Some(def) = def {
                    self.enqueue(def);
                } else if item.get("properties").is_some() {
                    children.push((format!("{child_path}.<name>"), item.clone()));
                }
            }
        }
        if let Some(item) = node
            .get("additionalProperties")
            .filter(|v| v.is_object())
            .filter(|_| path.is_empty())
        {
            let (resolved, def) = resolve(schema, item);
            let description = item["description"]
                .as_str()
                .or(resolved["description"].as_str())
                .unwrap_or("");
            let _ = writeln!(
                self.out,
                "| `<name>` | {} |  |  | {description} |",
                type_name(schema, item)
            );
            if let Some(def) = def {
                self.enqueue(def);
            }
        }
        for (child_path, child) in children {
            self.section(&format!("`{child_path}`"), &child_path, &child);
        }
    }

    /// A value that takes one of several shapes, such as a name that is one
    /// string or one per platform.
    fn shapes_section(&mut self, title: &str, node: &Value, branches: &[Value]) {
        let _ = writeln!(self.out, "\n### {title}\n");
        if let Some(description) = node["description"].as_str() {
            let _ = writeln!(self.out, "{description}\n");
        }
        let _ = writeln!(self.out, "One of:\n");
        let schema = self.schema;
        for branch in branches {
            let _ = writeln!(self.out, "- {}", type_name(schema, branch));
            if let (_, Some(def)) = resolve(schema, branch) {
                self.enqueue(def);
            }
        }
    }

    fn hotkey_section(&mut self, title: &str, node: &Value) {
        let _ = writeln!(self.out, "\n### {title}\n");
        if let Some(description) = node["description"].as_str() {
            let _ = writeln!(self.out, "{description}\n");
        }
        let modifiers = MODIFIER_TOKENS
            .iter()
            .map(|(name, _)| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let keys = KEY_TOKENS
            .iter()
            .map(|(name, _)| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            self.out,
            "A chord is one or more modifiers joined with `+`, then one key, all lowercase. \
             A function key from `f1` to `f20` can also stand alone.\n\n\
             - Modifiers: {modifiers}.\n\
             - Keys: a letter, a digit, or one of {keys}.\n\n\
             Or an object with a `macos` and a `windows` chord; a missing side means no binding there."
        );
    }

    fn enqueue(&mut self, def: String) {
        if !self.rendered.contains(&def) && !self.pending.contains(&def) {
            self.pending.push(def);
        }
    }
}

/// The node a property's schema stands for, following one `$ref`, and the
/// def's name when it was a ref.
fn resolve<'v>(schema: &'v Value, node: &'v Value) -> (&'v Value, Option<String>) {
    match node["$ref"]
        .as_str()
        .and_then(|r| r.strip_prefix("#/$defs/"))
    {
        Some(name) => (&schema["$defs"][name], Some(name.to_string())),
        None => (node, None),
    }
}

fn type_name(schema: &Value, property: &Value) -> String {
    let (resolved, def) = resolve(schema, property);
    if let Some(def) = def {
        if resolved.get("enum").is_some() {
            return "string".into();
        }
        let title = def_title(&def);
        return format!("[{title}](#{})", anchor(&title));
    }
    if let Some(item) = resolved
        .get("additionalProperties")
        .filter(|v| v.is_object())
    {
        return format!("map of {}", type_name(schema, item));
    }
    match &resolved["type"] {
        Value::String(name) => name.clone(),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" or "),
        _ => "object".into(),
    }
}

fn def_title(name: &str) -> String {
    match name {
        "Table" => "Extension table",
        "CommandSettings" => "Command settings",
        "AiCommandSettings" => "AI command settings",
        "AppSettings" => "Application settings",
        "AppName" => "Application name",
        "ProviderSettings" => "Provider settings",
        "PerPlatform" => "Per-platform value",
        "SnippetEntry" => "Snippet",
        "QuicklinkEntry" => "Quicklink",
        other => other,
    }
    .to_string()
}

fn anchor(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

fn default_of(node: &Value) -> String {
    match &node["default"] {
        Value::Null | Value::Object(_) => String::new(),
        Value::String(text) => format!("`{text}`"),
        Value::Array(items) => items
            .iter()
            .map(|item| format!("`{}`", item.as_str().unwrap_or_default()))
            .collect::<Vec<_>>()
            .join(", "),
        other => format!("`{other}`"),
    }
}

fn values_of(node: &Value) -> String {
    let Some(values) = node["enum"].as_array() else {
        return String::new();
    };
    let shown: Vec<String> = values
        .iter()
        .take(8)
        .map(|v| format!("`{}`", v.as_str().unwrap_or_default()))
        .collect();
    let mut text = shown.join(", ");
    if values.len() > shown.len() {
        let _ = write!(text, " and {} more", values.len() - shown.len());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::builtin_manifests;

    fn schema() -> Value {
        generate(UserFile::Config, &builtin_manifests())
    }

    fn validator() -> jsonschema::Validator {
        jsonschema::validator_for(&schema()).expect("the schema compiles")
    }

    fn instance(text: &str) -> Value {
        toml::from_str(text).expect("valid TOML")
    }

    fn accepts(text: &str) -> bool {
        validator().is_valid(&instance(text))
    }

    const FULL: &str = r#"
        version = 1
        hyperkey = { enabled = true, shift = false, tap = "escape" }
        quicklinks = { favicons = false }
        window-management = { commands = { left-half = { alias = "lh", hotkey = "hyper+left" } } }
        system = { commands = { open-config = { hotkey = "hyper+comma" } } }

        [launcher]
        hotkey = { macos = "cmd+space", windows = "ctrl+space" }
        autostart = true
        reset-after-seconds = 90

        [clipboard]
        enabled = true
        entries = 200
        excluded-applications = ["1Password"]
        primary-action = "copy"

        [applications.hotkeys]
        Safari = { hotkey = "hyper+b" }

        [applications.hotkeys.editor]
        name = { macos = "TextEdit", windows = "Notepad" }
        hotkey = "hyper+e"

        [ai]
        provider = "anthropic"
        thinking = "low"

        [ai.providers]
        anthropic = { kind = "anthropic", model = "claude-sonnet-5" }
        local = { kind = "ollama", base-url = "http://localhost:11434" }

        [ai.commands]
        make-longer = { enabled = false }

        [ai.commands.improve-writing]
        provider = "anthropic"
        thinking = "low"
        output = "paste"
        hotkey = "hyper+i"

        [ai.commands.review-rust]
        title = "Review Rust"
        prompt = "Review {{ selection }}"
        alias = "rr"
    "#;

    #[test]
    fn a_full_config_is_accepted() {
        let validator = validator();
        let full = instance(FULL);
        let errors: Vec<String> = validator
            .iter_errors(&full)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
        Config::parse(FULL).expect("the full sample parses");
    }

    #[test]
    fn reset_after_is_a_whole_number_of_seconds() {
        assert!(accepts(r#"launcher = { reset-after-seconds = 0 }"#));
        assert!(accepts(r#"launcher = { reset-after-seconds = 300 }"#));
        assert!(!accepts(r#"launcher = { reset-after-seconds = -5 }"#));
        assert!(!accepts(r#"launcher = { reset-after-seconds = 1.5 }"#));
    }

    #[test]
    fn a_misspelt_key_is_rejected() {
        assert!(!accepts(r#"launcher = { hotkeys = "alt+space" }"#));
        assert!(!accepts(r#"clipboard = { entry = 1 }"#));
    }

    #[test]
    fn an_unknown_top_level_table_is_rejected() {
        assert!(!accepts(r#"clipbaord = { entries = 1 }"#));
        assert!(accepts(
            r#"extensions = { "author.x" = { enabled = false, commands = { go = { alias = "g" } } } }"#
        ));
        assert!(!accepts(
            r#"extensions = { "author.x" = { colour = "red" } }"#
        ));
    }

    #[test]
    fn user_named_entries_are_accepted_and_checked() {
        assert!(accepts(
            r#"applications = { hotkeys = { Mine = { hotkey = "hyper+m" } } }"#
        ));
        assert!(!accepts(
            r#"applications = { hotkeys = { Mine = { hotkeys = "hyper+m" } } }"#
        ));
        assert!(accepts(
            r#"ai = { providers = { mine = { kind = "ollama" } } }"#
        ));
        assert!(accepts(
            r#"ai = { commands = { mine = { title = "T", prompt = "p" } } }"#
        ));
        assert!(!accepts(
            r#"ai = { commands = { mine = { prompts = "p" } } }"#
        ));
    }

    #[test]
    fn an_ai_only_setting_is_rejected_for_another_extension() {
        assert!(!accepts(
            r#"window-management = { commands = { left-half = { prompt = "p" } } }"#
        ));
        assert!(!accepts(
            r#"system = { commands = { lock = { output = "copy" } } }"#
        ));
    }

    #[test]
    fn a_command_dango_does_not_ship_is_rejected_outside_ai() {
        assert!(!accepts(
            r#"system = { commands = { reboot = { alias = "r" } } }"#
        ));
    }

    #[test]
    fn enumerated_values_are_checked() {
        assert!(!accepts(
            r#"ai = { providers = { x = { kind = "mistral" } } }"#
        ));
        assert!(!accepts(
            r#"ai = { commands = { x = { thinking = "deep" } } }"#
        ));
        assert!(!accepts(
            r#"ai = { commands = { x = { output = "speak" } } }"#
        ));
        assert!(!accepts(r#"ai = { thinking = "deep" }"#));
        assert!(!accepts(r#"clipboard = { primary-action = "speak" }"#));
        assert!(!accepts(r#"hyperkey = { key = "capslock" }"#));
        assert!(!accepts(r#"hyperkey = { tap = "lft" }"#));
        assert!(!accepts(r#"launcher = { display = "secondary" }"#));
        assert!(accepts(r#"launcher = { display = "primary" }"#));
    }

    #[test]
    fn every_token_matches_the_hotkey_pattern() {
        let validator = validator();
        let chord = |text: &str| {
            validator.is_valid(&json!({ "launcher": { "hotkey": text } }))
                && validator.is_valid(&json!({ "launcher": { "hotkey": { "windows": text } } }))
        };
        for (name, _) in MODIFIER_TOKENS {
            assert!(chord(&format!("{name}+a")), "{name}");
        }
        for (name, _) in KEY_TOKENS {
            assert!(chord(&format!("ctrl+{name}")), "{name}");
        }
        for ch in ('a'..='z').chain('0'..='9') {
            assert!(chord(&format!("hyper+{ch}")), "{ch}");
        }
        assert!(chord("ctrl+alt+shift+f12"));
        for n in 1..=20 {
            assert!(chord(&format!("f{n}")), "f{n}");
        }
        assert!(!chord("hyper+lft"));
        assert!(!chord("a"));
        assert!(!chord("space"));
        assert!(!chord("f21"));
        assert!(!chord("ctrl+f24"));
        assert!(!chord("ctrl+"));
        assert!(!chord("Hyper+Space"));
    }

    #[test]
    fn the_parser_and_the_pattern_agree_on_a_lone_key() {
        let validator = validator();
        let names = KEY_TOKENS.iter().map(|(name, _)| name.to_string());
        let singles = ('a'..='z').chain('0'..='9').map(String::from);
        let beyond = ["f21", "f24"].map(String::from);
        for key in names.chain(singles).chain(beyond) {
            assert_eq!(
                validator.is_valid(&json!({ "launcher": { "hotkey": key } })),
                crate::config::hotkey::parse(&key).is_ok(),
                "{key}"
            );
        }
    }

    #[test]
    fn every_built_in_table_and_command_is_named() {
        let schema = schema();
        let properties = &schema["properties"];
        for manifest in builtin_manifests() {
            let name = table_name(&manifest.id).unwrap();
            let table = &properties[name];
            assert!(table.is_object(), "{name}");
            for command in &manifest.commands {
                assert!(
                    table["properties"]["commands"]["properties"]
                        .get(&command.id)
                        .is_some(),
                    "{name}.{}",
                    command.id
                );
            }
        }
        let ai = properties["ai"]["properties"]["commands"]["properties"]
            .as_object()
            .unwrap();
        assert_eq!(ai.len(), 9, "the nine shipped AI commands");
        assert!(properties["ai"]["properties"].get("providers").is_some());
        assert!(properties["applications"]["properties"]
            .get("hotkeys")
            .is_some());
        assert!(properties["system"]["properties"].get("hotkeys").is_none());
    }

    const RESERVED_TOP_LEVEL: [&str; 4] = ["version", "launcher", "hyperkey", "extensions"];

    fn reserved_table_names<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        ids.into_iter()
            .filter_map(table_name)
            .filter(|name| RESERVED_TOP_LEVEL.contains(name))
            .collect()
    }

    #[test]
    fn no_built_in_takes_a_reserved_table_name() {
        let manifests = builtin_manifests();
        assert!(reserved_table_names(manifests.iter().map(|m| m.id.as_str())).is_empty());
        assert_eq!(reserved_table_names(["dango.launcher"]), ["launcher"]);
        let mut names: Vec<&str> = manifests
            .iter()
            .filter_map(|manifest| table_name(&manifest.id))
            .collect();
        names.sort_unstable();
        assert_eq!(names, crate::config::BUILT_IN_TABLES);
    }

    #[test]
    fn enabled_and_commands_mean_the_same_in_every_table() {
        let schema = schema();
        for name in crate::config::BUILT_IN_TABLES {
            let properties = &schema["properties"][name]["properties"];
            assert_eq!(properties["enabled"]["type"], "boolean", "{name}");
            if let Some(commands) = properties.get("commands") {
                assert_eq!(commands["type"], "object", "{name}");
                assert!(commands.get("properties").is_some(), "{name}");
            }
        }
    }

    #[test]
    fn a_setting_carries_its_type_default_and_values() {
        let schema = schema();
        let clipboard = &schema["properties"]["clipboard"]["properties"];
        assert_eq!(clipboard["entries"]["type"], "integer");
        assert_eq!(clipboard["entries"]["default"], 500);
        assert_eq!(clipboard["excluded-applications"]["type"], "array");
        assert_eq!(
            clipboard["excluded-applications"]["description"],
            "Applications whose copies are never recorded, matched ignoring case."
        );
        let action = resolve(&schema, &clipboard["primary-action"]).0;
        assert_eq!(action["enum"], json!(["paste", "copy"]));
        assert_eq!(
            schema["properties"]["quicklinks"]["properties"]["favicons"]["default"],
            true
        );
    }

    fn validator_for(file: UserFile) -> jsonschema::Validator {
        jsonschema::validator_for(&generate(file, &builtin_manifests())).expect("compiles")
    }

    #[test]
    fn a_record_under_any_id_is_checked_field_by_field() {
        let snippets = validator_for(UserFile::Snippets);
        assert!(snippets.is_valid(&instance(
            "version = 1\n[any-id]\nname = \"N\"\ntemplate = \"t\"\nkeyword = \";k\""
        )));
        assert!(!snippets.is_valid(&instance("[a]\nname = \"N\"\ntemplte = \"t\"")));
        assert!(!snippets.is_valid(&instance("[a]\nname = 3\ntemplate = \"t\"")));

        let quicklinks = validator_for(UserFile::Quicklinks);
        assert!(quicklinks.is_valid(&instance(
            "[docs]\nname = \"Docs\"\nurl = \"https://example.com\"\nbrowser = \"firefox\""
        )));
        assert!(!quicklinks.is_valid(&instance(
            "[docs]\nname = \"Docs\"\nurl = \"https://example.com\"\nbrowser = \"opera\""
        )));
    }

    #[test]
    fn a_record_field_carries_its_description() {
        let schema = generate(UserFile::Snippets, &builtin_manifests());
        let template = &schema["$defs"]["SnippetEntry"]["properties"]["template"];
        assert_eq!(template["type"], "string");
        assert!(template["description"]
            .as_str()
            .is_some_and(|text| text.starts_with("The text to insert.")));
    }

    #[test]
    fn an_auth_key_must_be_text() {
        let validator = validator_for(UserFile::Auth);
        assert!(validator.is_valid(&instance("version = 1\nanthropic = \"sk-ant\"")));
        assert!(!validator.is_valid(&instance("anthropic = 3")));
    }

    #[test]
    fn every_schema_uses_kebab_case_keys() {
        fn check(node: &Value, file: UserFile) {
            match node {
                Value::Object(map) => {
                    if let Some(Value::Object(properties)) = map.get("properties") {
                        for key in properties.keys() {
                            let kebab = key
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                            assert!(kebab, "{file:?}: {key}");
                        }
                    }
                    map.values().for_each(|child| check(child, file));
                }
                Value::Array(items) => items.iter().for_each(|child| check(child, file)),
                _ => {}
            }
        }
        for file in UserFile::ALL {
            check(&generate(file, &builtin_manifests()), file);
        }
    }

    #[test]
    fn writing_creates_replaces_and_leaves_an_identical_file_alone() {
        let dir = std::env::temp_dir().join(format!("dango-schema-{}", uuid::Uuid::new_v4()));
        let manifests = builtin_manifests();

        assert!(write_all(&dir, &manifests).is_empty());
        for file in UserFile::ALL {
            let path = dir.join(".schemas").join(file.schema_file_name());
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                render(&generate(file, &manifests))
            );
            let modified = std::fs::metadata(&path).unwrap().modified().unwrap();

            assert_eq!(write(&dir, file, &manifests), Ok(false));
            assert_eq!(
                std::fs::metadata(&path).unwrap().modified().unwrap(),
                modified
            );

            std::fs::write(&path, "{}").unwrap();
            assert_eq!(write(&dir, file, &manifests), Ok(true));
            assert_ne!(std::fs::read_to_string(&path).unwrap(), "{}");
        }
        assert!(!dir.join("config.schema.json").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_that_cannot_be_written_names_each_schema() {
        let dir = std::env::temp_dir().join(format!("dango-schema-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let errors = write_all(&blocker, &builtin_manifests());
        assert_eq!(errors.len(), UserFile::ALL.len());
        for (file, error) in UserFile::ALL.into_iter().zip(&errors) {
            assert!(error.contains(&file.schema_file_name()), "{error}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_settings_reference_is_current() {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/config.md"));
        let expected = reference(&builtin_manifests());
        if std::env::var_os("DANGO_UPDATE_DOCS").is_some() {
            std::fs::write(path, &expected).expect("write docs/config.md");
        }
        let actual = std::fs::read_to_string(path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            actual == expected,
            "docs/config.md is out of date. Run:\n  env DANGO_UPDATE_DOCS=1 cargo test the_settings_reference_is_current"
        );
    }

    #[test]
    fn the_reference_lists_every_property_once_per_shape() {
        let text = reference(&builtin_manifests());
        assert!(text.starts_with("# Dango configuration\n"));
        for file in UserFile::ALL {
            assert!(text.contains(&format!("\n## {}\n", file.file_name())));
        }
        assert!(text.contains("| `primary-action` | string | `paste` | `paste`, `copy` |"));
        assert!(text.contains("### `clipboard`"));
        assert!(!text.contains("preferences"));
        assert!(text.contains("### Command settings"));
        assert!(text.contains("### AI command settings"));
        assert!(text.contains("### Hotkey"));
        assert_eq!(text.matches("### Command settings").count(), 1);
    }
}
