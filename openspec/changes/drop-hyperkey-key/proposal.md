# Proposal

## Why

The `hyperkey.key` config field suggests any key can become the hyperkey, but
the hyperkey spec already makes CapsLock the only one. Every other value makes
the hyperkey fail to start ("is not a remappable key", tray shows the hyperkey
did not start). A field with one valid value is a trap, not a setting (#24).

## What Changes

- **BREAKING**: remove `key` from the `hyperkey` config block. A `hyperkey`
  block being present turns CapsLock into the hyperkey; `shift` and `tap` stay.
  No migration: Dango is alpha.
- The config schema no longer offers `key`, so an editor flags a leftover one.
  At runtime a leftover `key` is ignored like any other unknown key, so the
  hyperkey starts on CapsLock whatever it said.
- The "not a remappable key" failure goes away, because no config value can
  reach it.
- `docs/config.md` loses the `key` row.

## Non-goals

- Supporting another key as the hyperkey. If that is ever wanted, it comes back
  as its own change with a real allowlist and per-platform support.
- Changing `shift`, `tap`, live reload, or the permission behaviour.
- Changing the platform hyperkey code on either platform: the internal trigger
  stays, always CapsLock.

## Platforms

Windows and macOS alike. The change is in shared config parsing, the schema, and
the shared startup path; neither platform's hyperkey code changes, so there is
nothing platform-specific to verify by hand.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `hyperkey`: the configuration turns the hyperkey on or off and no longer names
  a key; a block with no key, or with a leftover one, makes CapsLock the
  hyperkey.

## Impact

- `src/config/mod.rs`: `Hyperkey::key` and `default_hyperkey_key` removed.
- `src/config/schema.rs`: `HYPERKEY_KEYS` and the `key` enum removed; tests and
  the full sample updated.
- `src/platform/mod.rs`: `HyperkeyTrigger::from_key` and its test removed.
- `src/app/mod.rs`: the hyperkey start path stops parsing a key and always uses
  CapsLock.
- `docs/config.md`: regenerated.
- The author's own `config.json` can drop its `key` line (optional, it is
  ignored).
