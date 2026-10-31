## Why

Since `replace-webview-with-slint`, Dango has no installer, no updater, and no
release workflow: `release.yml` still calls tauri-action and only runs by hand,
and the update source never finds anything. Builds are copied over by hand.
This change brings back tagged releases and in-app updates for the Slint app.

## What Changes

- Package with `cargo-packager`, configured in `Cargo.toml`:
  - Windows: one NSIS installer that installs per user and needs no admin
  - macOS on Apple Silicon: a `.app` in a `.dmg`, plus the `.app.tar.gz` the
    updater installs
  - bundle identifier stays `com.nimayer.dango`, so macOS keeps its
    Accessibility and Input Monitoring grants
- **BREAKING** No more MSI. The Windows release carries the NSIS installer
  only.
- Sign the macOS app with the existing certificate from the repository
  secrets: same identity every release, no notarization, so the one-time
  Gatekeeper override stays.
- Sign the update artifacts with the existing minisign key, under its current
  secret names. The public key built into Dango is the one the Tauri build
  used.
- Implement `UpdateSource` with `cargo-packager-updater` in place of
  `DisabledSource`. The check, the download, and the install behave as
  `app-updates` already describes.
- Rewrite `release.yml`: a `v*` tag builds both platforms, creates a draft
  release with the changelog section, attaches the installers, the updater
  artifacts, their signatures, and one `latest.json`, and publishes only when
  both builds succeeded.
- `AGENTS.md`: drop the "updater and release workflow are off" fact and add the
  packaging facts.

## Non-goals

- Windows code signing. SmartScreen warns on first install.
- macOS notarization, Intel or universal macOS builds.
- Delta updates, update channels, or pre-release updates.
- A migration from the old Tauri install. The author uninstalls it by hand.
- Changes to the release command, versioning, or the changelog.
- Linux.

## Platforms

Windows (x64) and macOS (Apple Silicon). Intel Macs are not covered, as today,
because the author has none. Each platform's install and update is checked by
hand on that platform.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `release-pipeline`: the Windows release carries the NSIS installer only, and
  it installs per user without admin rights.

`app-updates` keeps its requirements unchanged; this change makes them true
again.

## Impact

- Code: `src/extensions/updates/source.rs` (new packager source),
  `src/slint_shell/mod.rs` (wires it in place of `DisabledSource`), the restart
  after a macOS install.
- Dependencies: add `cargo-packager-updater`. `cargo-packager` is a CI and
  local tool, pinned through mise.
- Config: `Cargo.toml` (`[package.metadata.packager]`), `Info.plist`,
  `Entitlements.plist`, `icons/`.
- CI: `.github/workflows/release.yml` rewritten; `mise.toml`.
- Docs: `AGENTS.md`, `README.md` (install steps).
