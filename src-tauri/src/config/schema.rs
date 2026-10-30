//! The JSON Schema for the config file, generated from the structs that read
//! it and from the built-in manifests, so an editor's completions and the
//! settings reference cannot drift from what the code accepts.

use std::fmt::Write as _;
use std::path::Path;

use serde_json::{json, Map, Value};

use super::hotkey::{KEY_TOKENS, MODIFIER_TOKENS};
use super::Config;
use crate::extension::{Manifest, PreferenceDecl, PreferenceKind};

pub const FILE_NAME: &str = "config.schema.json";

const AI_ID: &str = "dango.ai";
const APPLICATIONS_ID: &str = "dango.applications";
/// Command settings only the AI extension reads; every other extension's
/// commands reject them.
const AI_ONLY: &[&str] = &["title", "prompt", "provider", "model", "thinking", "output"];
const HYPERKEY_KEYS: &[&str] = &["capslock", "caps"];

pub fn generate(manifests: &[Manifest]) -> Value {
    let mut schema = schemars::schema_for!(Config).to_value();
    strip_null(&mut schema);
    close_objects(&mut schema);
    describe_root(&mut schema);
    constrain_hotkeys(&mut schema);
    constrain_hyperkey(&mut schema);
    split_command_settings(&mut schema);
    let extensions = extensions(manifests, &schema["$defs"]["Extension"]);
    schema["properties"]["extensions"] = extensions;
    defs(&mut schema).remove("Extension");
    tidy(&mut schema);
    schema
}

pub fn render(schema: &Value) -> String {
    let mut text = serde_json::to_string_pretty(schema).unwrap_or_default();
    text.push('\n');
    text
}

/// Writes the schema into `dir` unless the copy there already matches.
/// Returns whether a write happened.
pub fn write(dir: &Path, manifests: &[Manifest]) -> Result<bool, String> {
    let path = dir.join(FILE_NAME);
    let text = render(&generate(manifests));
    if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
        return Ok(false);
    }
    std::fs::create_dir_all(dir)
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
    format!("^(?:(?:{modifiers})\\+)+(?:{keys}|[a-z0-9])$")
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
    hyperkey["key"]["enum"] = json!(HYPERKEY_KEYS);
    hyperkey["tap"]["enum"] = json!(key_names());
}

/// One shape for every extension's commands and a wider one for the AI
/// extension, whose commands are also defined in the file.
fn split_command_settings(schema: &mut Value) {
    let defs = defs(schema);
    let full = defs["CommandSettings"].clone();
    defs.insert("AiCommandSettings".into(), full);
    let generic = defs["CommandSettings"]["properties"]
        .as_object_mut()
        .expect("command settings has properties");
    for key in AI_ONLY {
        generic.remove(*key);
    }
}

fn extensions(manifests: &[Manifest], base: &Value) -> Value {
    let mut properties = Map::new();
    for manifest in manifests {
        let mut entry = base.clone();
        entry["description"] = json!(format!("Settings for the {} extension.", manifest.name));
        let fields = entry["properties"]
            .as_object_mut()
            .expect("extension has properties");
        let described =
            |fields: &Map<String, Value>, name: &str| fields[name]["description"].clone();
        let preferences = preferences(&manifest.preferences, described(fields, "preferences"));
        let commands = commands(manifest, described(fields, "commands"));
        fields.insert("preferences".into(), preferences);
        fields.insert("commands".into(), commands);
        if manifest.id != APPLICATIONS_ID {
            fields.remove("apps");
        }
        if manifest.id != AI_ID {
            fields.remove("providers");
        }
        properties.insert(manifest.id.clone(), entry);
    }
    json!({
        "type": "object",
        "description": "Per-extension settings, keyed by extension id.",
        "additionalProperties": false,
        "properties": properties
    })
}

fn preferences(declarations: &[PreferenceDecl], description: Value) -> Value {
    let mut properties = Map::new();
    for decl in declarations.iter().filter(|d| d.command_id.is_none()) {
        let mut property = json!({
            "type": kind_name(decl.kind),
            "description": decl.description
        });
        if let Some(default) = &decl.default {
            property["default"] = default.clone();
        }
        if !decl.values.is_empty() {
            property["enum"] = json!(decl.values);
        }
        properties.insert(decl.key.clone(), property);
    }
    json!({
        "type": "object",
        "description": description,
        "additionalProperties": false,
        "properties": properties
    })
}

fn kind_name(kind: PreferenceKind) -> &'static str {
    match kind {
        PreferenceKind::String => "string",
        PreferenceKind::Number => "number",
        PreferenceKind::Boolean => "boolean",
    }
}

/// Commands are named by id, not title: a title can differ per platform, and
/// the reference generated from this must not.
fn commands(manifest: &Manifest, description: Value) -> Value {
    let item = if manifest.id == AI_ID {
        "#/$defs/AiCommandSettings"
    } else {
        "#/$defs/CommandSettings"
    };
    let mut properties = Map::new();
    for command in &manifest.commands {
        properties.insert(
            command.id.clone(),
            json!({
                "$ref": item,
                "description": format!("Settings for the {} command.", command.id)
            }),
        );
    }
    let open = manifest.id == AI_ID;
    json!({
        "type": "object",
        "description": description,
        "additionalProperties": if open { json!({ "$ref": item }) } else { json!(false) },
        "properties": properties
    })
}

/// The settings reference as Markdown: one table per object, every property
/// with its type, default, allowed values, and description. Shared shapes
/// from `$defs` are rendered once and linked to.
pub fn reference(schema: &Value) -> String {
    let mut out = String::from(
        "# Dango configuration\n\nEvery setting in config.json. Generated from the code; \
         do not edit by hand.\n",
    );
    let mut writer = Reference {
        schema,
        out: &mut out,
        pending: Vec::new(),
        rendered: Vec::new(),
    };
    writer.section("Top level", "", schema);
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
        let _ = writeln!(self.out, "\n## {title}\n");
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
                default_of(resolved),
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
        for (child_path, child) in children {
            self.section(&format!("`{child_path}`"), &child_path, &child);
        }
    }

    /// A value that takes one of several shapes, such as a name that is one
    /// string or one per platform.
    fn shapes_section(&mut self, title: &str, node: &Value, branches: &[Value]) {
        let _ = writeln!(self.out, "\n## {title}\n");
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
        let _ = writeln!(self.out, "\n## {title}\n");
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
            "A chord is one or more modifiers joined with `+`, then one key, all lowercase.\n\n\
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
        "CommandSettings" => "Command settings",
        "AiCommandSettings" => "AI command settings",
        "AppSettings" => "Application settings",
        "AppName" => "Application name",
        "ProviderSettings" => "Provider settings",
        "PerPlatform" => "Per-platform value",
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
        Value::Null => String::new(),
        Value::String(text) => format!("`{text}`"),
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
        generate(&builtin_manifests())
    }

    fn validator() -> jsonschema::Validator {
        jsonschema::validator_for(&schema()).expect("the schema compiles")
    }

    fn accepts(text: &str) -> bool {
        validator().is_valid(&serde_json::from_str(text).expect("valid JSON"))
    }

    const FULL: &str = r#"{
        "$schema": "./config.schema.json",
        "version": 1,
        "launcher": { "hotkey": { "macos": "cmd+space", "windows": "ctrl+space" }, "autostart": true, "reset-after": 90 },
        "hyperkey": { "key": "capslock", "shift": false, "tap": "escape" },
        "selection": { "excluded-applications": "Zed" },
        "extensions": {
            "dango.clipboard": {
                "enabled": true,
                "preferences": { "entries": 200, "excluded-applications": "1Password", "primary-action": "copy" }
            },
            "dango.quicklinks": { "preferences": { "favicons": false } },
            "dango.applications": {
                "apps": {
                    "Safari": { "hotkey": "hyper+b" },
                    "editor": { "name": { "macos": "TextEdit", "windows": "Notepad" }, "hotkey": "hyper+e" }
                }
            },
            "dango.ai": {
                "providers": {
                    "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" },
                    "claude": { "kind": "cli", "command": "claude", "args": ["-p"] },
                    "local": { "kind": "ollama", "baseUrl": "http://localhost:11434" }
                },
                "preferences": { "provider": "anthropic", "thinking": "low" },
                "commands": {
                    "improve-writing": { "provider": "anthropic", "thinking": "low", "output": "paste", "hotkey": "hyper+i" },
                    "make-longer": { "enabled": false },
                    "review-rust": { "title": "Review Rust", "prompt": "Review {{ selection }}", "alias": "rr" }
                }
            },
            "dango.window-management": {
                "commands": { "left-half": { "alias": "lh", "hotkey": "hyper+left" } }
            },
            "dango.system": {
                "commands": { "open-config": { "hotkey": "hyper+comma" } }
            }
        }
    }"#;

    #[test]
    fn a_full_config_is_accepted() {
        let validator = validator();
        let instance: Value = serde_json::from_str(FULL).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
        Config::parse(FULL).expect("the full sample parses");
    }

    #[test]
    fn a_full_config_survives_a_round_trip() {
        let written = Config::parse(FULL).unwrap().to_json();
        let original: Value = serde_json::from_str(FULL).unwrap();
        let again: Value = serde_json::from_str(&written).unwrap();
        assert_eq!(again, original);
    }

    #[test]
    fn reset_after_is_a_whole_number_of_seconds() {
        assert!(accepts(r#"{ "launcher": { "reset-after": 0 } }"#));
        assert!(accepts(r#"{ "launcher": { "reset-after": 300 } }"#));
        assert!(!accepts(r#"{ "launcher": { "reset-after": -5 } }"#));
        assert!(!accepts(r#"{ "launcher": { "reset-after": 1.5 } }"#));
    }

    #[test]
    fn a_misspelt_key_is_rejected() {
        assert!(!accepts(r#"{ "launcher": { "hotkeys": "alt+space" } }"#));
        assert!(!accepts(
            r#"{ "extensions": { "dango.clipboard": { "preferences": { "entry": 1 } } } }"#
        ));
    }

    #[test]
    fn an_extension_dango_does_not_ship_is_rejected() {
        assert!(!accepts(
            r#"{ "extensions": { "dango.future": { "enabled": true } } }"#
        ));
    }

    #[test]
    fn user_named_entries_are_accepted_and_checked() {
        assert!(accepts(
            r#"{ "extensions": { "dango.applications": { "apps": { "Mine": { "hotkey": "hyper+m" } } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.applications": { "apps": { "Mine": { "hotkeys": "hyper+m" } } } } }"#
        ));
        assert!(accepts(
            r#"{ "extensions": { "dango.ai": { "providers": { "mine": { "kind": "ollama" } } } } }"#
        ));
        assert!(accepts(
            r#"{ "extensions": { "dango.ai": { "commands": { "mine": { "title": "T", "prompt": "p" } } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.ai": { "commands": { "mine": { "prompts": "p" } } } } }"#
        ));
    }

    #[test]
    fn an_ai_only_setting_is_rejected_for_another_extension() {
        assert!(!accepts(
            r#"{ "extensions": { "dango.window-management": { "commands": { "left-half": { "prompt": "p" } } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.system": { "commands": { "lock": { "output": "copy" } } } } }"#
        ));
    }

    #[test]
    fn a_command_dango_does_not_ship_is_rejected_outside_ai() {
        assert!(!accepts(
            r#"{ "extensions": { "dango.system": { "commands": { "reboot": { "alias": "r" } } } } }"#
        ));
    }

    #[test]
    fn enumerated_values_are_checked() {
        assert!(!accepts(
            r#"{ "extensions": { "dango.ai": { "providers": { "x": { "kind": "mistral" } } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.ai": { "commands": { "x": { "thinking": "deep" } } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.ai": { "commands": { "x": { "output": "speak" } } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.ai": { "preferences": { "thinking": "deep" } } } }"#
        ));
        assert!(!accepts(
            r#"{ "extensions": { "dango.clipboard": { "preferences": { "primary-action": "speak" } } } }"#
        ));
        assert!(!accepts(r#"{ "hyperkey": { "key": "tab" } }"#));
        assert!(!accepts(r#"{ "hyperkey": { "tap": "lft" } }"#));
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
        assert!(!chord("hyper+lft"));
        assert!(!chord("space"));
        assert!(!chord("ctrl+"));
        assert!(!chord("Hyper+Space"));
    }

    #[test]
    fn every_built_in_preference_and_command_is_named() {
        let schema = schema();
        let extensions = &schema["properties"]["extensions"]["properties"];
        let manifests = builtin_manifests();
        assert_eq!(extensions.as_object().unwrap().len(), manifests.len());
        for manifest in &manifests {
            let entry = &extensions[&manifest.id];
            for decl in &manifest.preferences {
                let property = &entry["properties"]["preferences"]["properties"][&decl.key];
                assert_eq!(property["type"], kind_name(decl.kind), "{}", decl.key);
                assert_eq!(property["description"], decl.description, "{}", decl.key);
                match &decl.default {
                    Some(Value::Number(number)) => {
                        assert_eq!(
                            property["default"].as_f64(),
                            number.as_f64(),
                            "{}",
                            decl.key
                        )
                    }
                    other => {
                        assert_eq!(&property["default"], other.as_ref().unwrap_or(&Value::Null))
                    }
                }
                if decl.values.is_empty() {
                    assert!(property.get("enum").is_none());
                } else {
                    assert_eq!(property["enum"], json!(decl.values));
                }
            }
            for command in &manifest.commands {
                assert!(
                    entry["properties"]["commands"]["properties"]
                        .get(&command.id)
                        .is_some(),
                    "{}.{}",
                    manifest.id,
                    command.id
                );
            }
        }
        let ai = extensions[AI_ID]["properties"]["commands"]["properties"]
            .as_object()
            .unwrap();
        assert_eq!(ai.len(), 9, "the nine shipped AI commands");
        assert!(extensions[AI_ID]["properties"].get("providers").is_some());
        assert!(extensions[APPLICATIONS_ID]["properties"]
            .get("apps")
            .is_some());
        assert!(extensions["dango.system"]["properties"]
            .get("apps")
            .is_none());
    }

    #[test]
    fn writing_creates_replaces_and_leaves_an_identical_file_alone() {
        let dir = std::env::temp_dir().join(format!("dango-schema-{}", uuid::Uuid::new_v4()));
        let manifests = builtin_manifests();
        let path = dir.join(FILE_NAME);

        assert_eq!(write(&dir, &manifests), Ok(true));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            render(&generate(&manifests))
        );
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();

        assert_eq!(write(&dir, &manifests), Ok(false));
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );

        std::fs::write(&path, "{}").unwrap();
        assert_eq!(write(&dir, &manifests), Ok(true));
        assert_ne!(std::fs::read_to_string(&path).unwrap(), "{}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_that_cannot_be_written_names_the_path() {
        let dir = std::env::temp_dir().join(format!("dango-schema-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let error = write(&blocker, &builtin_manifests()).unwrap_err();
        assert!(error.contains(FILE_NAME), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_settings_reference_is_current() {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/config.md"));
        let expected = reference(&schema());
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
        let text = reference(&schema());
        assert!(text.starts_with("# Dango configuration\n"));
        assert!(text.contains("| `primary-action` | string | `paste` | `paste`, `copy` |"));
        assert!(text.contains("## `extensions.dango.clipboard.preferences`"));
        assert!(text.contains("## Command settings"));
        assert!(text.contains("## AI command settings"));
        assert!(text.contains("## Hotkey"));
        assert_eq!(text.matches("## Command settings").count(), 1);
    }
}
