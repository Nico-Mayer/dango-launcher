## Why

The release pipeline produces installers, but an installed Dango never learns
that a newer one exists. Updating means opening GitHub, downloading, and
reinstalling by hand on both machines. Dango should find and install its own
updates.

On macOS there is a second problem. The current builds are ad-hoc signed, so
their designated requirement is the binary's hash and changes with every build.
macOS ties Accessibility and Input Monitoring grants to it, so every update,
manual or automatic, can look like a new app. Self-update is only pleasant if the
macOS build keeps one signing identity across releases.

## What Changes

- Dango checks GitHub for a newer release shortly after startup and again every
  few hours while it runs. Development builds never check.
- When an update is available, an item at the top of root search names the new
  version. Its primary action downloads it, verifies its signature, installs it,
  and restarts Dango. The tray menu shows the same item.
- A "Check for Updates" command checks on demand and reports the result.
- Dango installs an update only after the user confirms. It never installs on
  its own.
- The release workflow signs the updater artifacts and publishes a `latest.json`
  manifest on each GitHub release. On Windows the manifest points at the NSIS
  installer.
- The macOS build is signed with a stable code signing certificate held as a CI
  secret, instead of ad-hoc. No notarization.

Platforms: Windows and macOS, both fully covered.

## Non-goals

- Installing without the user's confirmation, or in the background on quit.
- Delta updates, release channels, pre-releases, rollback, or downgrades.
- Notarization or a Developer ID certificate. Gatekeeper still needs the
  one-time override on first install.
- Distribution through winget, Scoop, or Homebrew.
- A setting to turn the automatic check off.
- Showing the release notes inside Dango.
- Updating the MSI install. The NSIS installer is the update route on Windows.

## Capabilities

### New Capabilities

- `app-updates`: when Dango checks for an update, how an available update is
  shown in root search and the tray, the "Check for Updates" command, the
  confirm, download, install, and restart flow, failure reporting, and
  development builds never checking.

### Modified Capabilities

- `release-pipeline`: "A tag becomes a GitHub release with installers" adds
  signed updater artifacts and the `latest.json` manifest to every release, and
  a stable macOS signing identity.

## Impact

- `src-tauri/Cargo.toml`: `tauri-plugin-updater` and `tauri-plugin-process`.
- `src-tauri/tauri.conf.json`: `createUpdaterArtifacts`, the updater `pubkey`,
  `endpoints`, Windows `installMode`, macOS signing and entitlements.
- `src-tauri/src/extensions/`: a new `updates` built-in extension with a root
  provider and the command.
- `src-tauri/src/lib.rs`: the periodic check and the tray item.
- `.github/workflows/release.yml`: updater signing key and macOS certificate
  secrets, `uploadUpdaterJson: true`.
- GitHub repository secrets: the updater private key and its password, the
  macOS certificate, its password, and the signing identity.
- No change to the config file, the extension manifest, the view protocol, or
  stored data.
