# Proposal

## Why

On macOS, Finder can't be found in root search, and neither can Archive
Utility, Keychain Access, or About This Mac. They live under
`/System/Library/CoreServices/`, which the applications index never reads.

## What Changes

- On macOS, the applications index also covers `Finder.app` and the
  user-facing applications folder under `/System/Library/CoreServices/`.
- The rest of `/System/Library/CoreServices/` stays out. It holds about a
  hundred background agents and helpers (Dock, loginwindow, WiFiAgent) that no
  user launches by name. Finder is the one bundle there that is added, by
  name. Other user-facing ones (Screen Time, Siri, Software Update) stay out
  for now.

Platforms: macOS only. On Windows, discovery already goes through
`shell:AppsFolder`, which lists File Explorer and the system tools. Windows
needs no change.

## Non-goals

- New system commands (restart, shut down, log out, sleep displays). Those are
  a separate change.
- Indexing the rest of `/System/Library/CoreServices/`, or `/System/Library/`
  in general.
- User-configurable extra search folders.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `applications`: "Discovery on macOS" names Finder and the system's core
  services applications folder as well.

## Impact

- `src/platform/macos/apps.rs`: the indexer's roots, plus the single Finder
  bundle.
- `openspec/specs/applications/spec.md`: the macOS discovery requirement.
- No change to the extension manifest, view protocol, config, or stored data.
