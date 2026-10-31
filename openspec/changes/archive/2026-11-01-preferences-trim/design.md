## Context

See proposal.md for why. Current state, checked against the code:

- `PreferenceDecl` (`src/extension/manifest.rs`) has `command_id: Option<String>`
  and `required: bool`. Every built-in sets `None` and `false` (clipboard,
  snippets/quicklinks, ai).
- `PreferenceDecl::effective` and `is_unsatisfied` are called only by their
  own tests.
- `Preferences` (`src/extension/preferences.rs`) reads and writes with
  `command_id = None` only. The scoped `read` is private; only a test passes a
  command id.
- `PreferenceStore::get`/`set` take `command_id: Option<&str>`. `FileConfig`
  (`src/config/store.rs`) maps `Some` to `CommandSettings.preferences`, and
  `Config::preference` does the same on read. The only outside callers
  (`src/extensions/ai/commands.rs`) pass `None`.
- `CommandSettings.preferences` (`src/config/mod.rs`) is never read with a
  command id. AI commands carry their own typed fields instead.
- `src/config/schema.rs` filters out command-scoped declarations.

## Goals / Non-Goals

**Goals:** one preference scope (the extension) and no required flag, in the
manifest, the store trait, and the config file.

**Non-Goals:** any change to extension-level read, default, value-set, or
write-back behaviour.

## Decisions

### Remove the command scope end to end

Drop `command_id` from `PreferenceDecl`, from `PreferenceStore::get`/`set`,
and from `Config::preference`, and drop `CommandSettings.preferences`. Keeping
the trait parameter or the config field would leave a path that is always
`None` or never read.

Alternative rejected: trim only the manifest and leave the store and config
field. That keeps a second, unreachable copy of the same idea and a config key
the schema advertises but nothing honours.

`FileConfig::set` keeps writing and persisting extension-level values exactly
as now. Only its unreachable command branch goes.

### Remove `required`, `effective`, `is_unsatisfied`

No built-in sets `required`, and no caller runs the check. `Preferences::read`
already does the default fallback that `effective` describes.

Alternative rejected: keep them for future third-party extensions. The
sandboxed host does not exist yet. The field can come back with the first
extension that needs it, with a real prompt flow behind it.

### Manifest contract

This changes the extension manifest, a versioned contract: a preference
declaration loses two fields. `SUPPORTED_MANIFEST_VERSION` stays `1`. The
project is alpha, manifests are Rust literals in the built-ins, and no
third-party manifest exists to read, so a bump would protect nothing. The
view protocol is not touched.

### Config file

`extensions.<id>.commands.<command>.preferences` leaves the schema and the
generated `docs/config.md`. A file that still has it parses (unknown keys are
kept in `extra`) and the schema flags it as unknown. No migration, per the
alpha rule.

## Risks / Trade-offs

- [A future extension needs a per-command or required preference] → Add it
  back then, with a spec scenario and an implementation in the same change.
- [A user config holds a command `preferences` key] → It had no effect before
  either; the schema now points it out.
