## Context

See proposal.md for why. Requirements are in `specs/app-updates` and
`specs/release-pipeline`.

Current state:

- `release.yml` builds with `tauri-apps/tauri-action@v1` into a draft release,
  sets `uploadUpdaterJson: false`, and publishes only after both builds pass.
- The repository is public, so release assets are reachable without auth.
- macOS bundles are ad-hoc and linker-signed. v0.3.1 and v0.4.0 were checked
  with `codesign -d -r-`: the designated requirement is `cdhash H"..."`, and the
  signing identifier is a random `dango-<hash>`, different in every build.
- Root search already pins items above an empty query (`Candidate::pinned`,
  kept in provider order). The `permissions` extension uses it for its
  warnings, and `lib.rs` keeps tray items in step with a live handle.
- A no-view command's success hides the launcher with no message. Showing a
  result needs a view. The detail view has `markdown` and `loading`, and the
  view protocol replaces the full tree on each update.

## Goals / Non-Goals

**Goals:**

- Use the official updater with the GitHub release as the only server.
- One `updates` built-in extension owns the check state, the root item, the
  command, and the install flow. `lib.rs` only schedules the check.
- No change to the extension manifest, the view protocol, or stored data.

**Non-Goals:**

- Abstracting the updater behind a platform trait. The plugin already hides
  the platform difference. The only per-platform part is config.

## Decisions

### Updater: `tauri-plugin-updater` with a static `latest.json`

The endpoint is
`https://github.com/Nico-Mayer/dango-launcher/releases/latest/download/latest.json`.
GitHub resolves `latest` to the newest published, non-draft, non-prerelease
release. That matches the spec rule that only published releases count, and the
workflow already publishes last, so a half-built release is never served.

Alternatives rejected:

- **Velopack**: delta updates and its own installer. It replaces the Tauri
  bundler and NSIS setup. Too much for a two-machine hobby app.
- **cargo-packager-updater / CrabNebula Cloud**: same model as the plugin with
  an extra vendor and no gain.
- **A dynamic update server**: needed only for staged rollout or channels, which
  are non-goals.
- **winget / Scoop / Homebrew**: not self-update. Possible later as an extra.

### Driven from Rust, not the JavaScript API

The check, download, and install run in Rust through `UpdaterExt`. The frontend
renders views as for any other extension, so no updater permission goes into
`capabilities/default.json`. `tauri-plugin-process` is added only for
`AppHandle::restart` after the macOS install. On Windows the plugin exits the
app itself before the installer runs.

The extension talks to a small `UpdateSource` trait (check, download and
install) instead of the plugin directly. The plugin implements it in
production and a fake implements it in tests, so the state, the root item, and
the streamed views are covered by `cargo test` without a network.

The check and download run with `tokio::spawn`, so every Tauri command on this
path is `async fn` (see AGENTS.md build facts).

### Scheduling

`lib.rs` starts one task after setup. It waits briefly, then checks, then
sleeps 6 hours between checks. The delay keeps the check off startup and the
hotkey path. `cfg!(debug_assertions)` disables the task and makes the command
return the development-build view, the same meaning the autostart and the
development indicator use.

The result is stored in the extension as `Option<Update>`, behind a mutex. A
failed automatic check logs and leaves the previous state.

### Surfaces

- **Root item**: a pinned candidate from the `updates` root provider. The
  `permissions` extension is registered before `updates`, so permission
  warnings come first.
- **Install flow**: the action returns `ActionOutcome::Started` and streams a
  detail view. While downloading, the view has `loading: true` and its markdown
  shows the downloaded and total size. The full tree is replaced on each
  progress step, at a coarse step so it does not flood the channel. A failure
  is streamed as the view's final state.
- **Check for Updates**: a `view` command that streams a detail view: checking,
  then up to date, available (with "Install and restart"), or failed.

### Windows install route

`plugins.updater.windows.installMode` is `passive`. tauri-action has to put the
NSIS `.exe` in `latest.json`, not the `.msi`. NSIS installs per user, so there
is no admin prompt, and the path stays the same, so autostart keeps working.
The workflow sets the action's NSIS preference explicitly, because its default
is not verified.

### macOS signing

Use standard Tauri signing: tauri-action reads `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, and `APPLE_SIGNING_IDENTITY` from secrets. Leave
the Apple ID and API key variables unset so notarization is skipped. The
certificate is the author's existing Apple Development certificate. Tauri
signs with the bundle identifier, so the designated requirement becomes
`identifier "com.nimayer.dango" and anchor apple generic and certificate
leaf[subject.CN] = ...`, stable while the certificate is.

Signing with a real identity turns on the hardened runtime. Under it, Empty
Trash's `osascript` call to Finder needs the
`com.apple.security.automation.apple-events` entitlement and an
`NSAppleEventsUsageDescription`. During the spike, tccd logged this exact
denial for a re-signed build. Add an `Entitlements.plist` and the Info.plist
key, and set `bundle.macOS.entitlements`.

Alternatives rejected:

- **Keep ad-hoc signing**: grants can reset on every update.
- **Self-signed certificate**: no Apple account needed, but untested, and
  Apple Development is already in the keychain.
- **Developer ID and notarization**: removes the Gatekeeper override but needs
  the paid program and more CI steps. A non-goal for now.

### Updater signing key

Generate with `npx tauri signer generate` and keep it outside the repo. The
private key and password go into `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The public key goes into
`tauri.conf.json`. `createUpdaterArtifacts: true` produces `Dango.app.tar.gz`
on macOS and a signed NSIS `.exe` on Windows.

### Interface text

Written with the `ux-writing` skill and checked against
`openspec/specs/interface-copy`.

| Surface | Text |
|---|---|
| Root item title | "Dango 0.5.0 is available" |
| Root item subtitle | "You have 0.4.0" |
| Primary action | "Install and restart" |
| Command title | "Check for Updates" |
| Check view, in progress | "Checking for updates" |
| Check view, up to date | "You have the latest version" |
| Download view | "Downloading Dango 0.5.0" |
| Signature failure | "Couldn't verify the update, so it wasn't installed. Try again later." |
| Download failure | "Couldn't download the update. Check your connection and try again." |
| Check failure | "Couldn't check for updates. Check your connection and try again." |
| Development build | "Updates are off in development builds." |

## Risks / Trade-offs

- [Unresolved: Accessibility on a re-signed build] In the spike, a build
  re-signed with the Apple Development certificate got Input Monitoring and
  Post Event grants, but `AXIsProcessTrusted` stayed false with the toggle on,
  even after reset, re-adding, and restart. The likely factor was several
  `Dango.app` bundles with the same identifier, but this was not proven.
  → A manual task tests a real CI-signed release on a Mac with no other Dango
  bundles. If it still fails, try a self-signed certificate next.
- [Apple Development certificate expires yearly] Renewing changes the leaf and
  resets grants once. → Accept for one user. Update the secret when renewing.
- [Losing the updater private key] Installed builds can no longer verify
  updates. → Store it in the password manager. A one-time manual reinstall is
  the recovery.
- [A broken release ships and installs itself] → Updates install only on
  confirmation, and the previous installer stays on GitHub for a manual
  downgrade.
- [GitHub rate limits] Unauthenticated release downloads are served from a CDN.
  Two machines checking every 6 hours is far below any limit.

## Migration Plan

The first release with the updater has to be installed by hand on both
machines. Installs before it do not have the updater. Every release after it
updates itself.

## Open Questions

- Does tauri-action prefer NSIS in `latest.json` by default? The workflow sets
  it explicitly either way.
