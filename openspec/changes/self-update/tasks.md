## 1. Signing keys and secrets

- [x] 1.1 Generate the updater key pair with `npx tauri signer generate`, store the private key and password in the password manager, and add them as the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets; verify with `gh secret list`
- [x] 1.2 Export the Apple Development certificate as a `.p12`, and add it base64-encoded with its password and identity as the `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, and `APPLE_SIGNING_IDENTITY` secrets; verify with `gh secret list`

## 2. Release pipeline

- [x] 2.1 In `src-tauri/tauri.conf.json`, set `bundle.createUpdaterArtifacts`, the updater `pubkey`, the `latest.json` endpoint, and `windows.installMode: passive`; verify a local `npx tauri build --no-bundle` still succeeds
- [x] 2.2 Add `src-tauri/Entitlements.plist` with `com.apple.security.automation.apple-events`, an `NSAppleEventsUsageDescription` in the macOS Info.plist, and `bundle.macOS.entitlements`; verify a local signed bundle shows the entitlement in `codesign -d --entitlements -`
- [ ] 2.3 In `.github/workflows/release.yml`, pass the updater and Apple secrets to tauri-action, set `uploadUpdaterJson: true`, and prefer NSIS in the manifest; verify with `actionlint` or a successful CI run on a test tag
- [ ] 2.4 Cut a release and verify its assets: `latest.json` names the version with a Windows NSIS and a macOS `.app.tar.gz` entry, `releases/latest/download/latest.json` serves it, and `codesign -d -r-` on the macOS bundle shows `identifier "com.nimayer.dango"` and the certificate, not a `cdhash`

## 3. Update source

- [x] 3.1 Add `tauri-plugin-updater` and `tauri-plugin-process` to `src-tauri/Cargo.toml` and register both in `lib.rs`; verify with `cargo build`
- [x] 3.2 Add the `UpdateSource` trait (check, download with progress, install) and a fake for tests under `src-tauri/src/extensions/updates/`; verify with `cargo build`
- [x] 3.3 Implement `UpdateSource` over `UpdaterExt`, restarting with `tauri-plugin-process` after the install on macOS; verify with `cargo clippy -- -D warnings`

## 4. Updates extension

- [x] 4.1 Add the `updates` built-in extension with the update state and a root provider returning one pinned item "Dango X is available" with subtitle "You have Y" and keywords "update" and "Dango", registered after `permissions`; verify with unit tests against the fake for no update, an update, and ordering after a permission warning via `cargo test updates`
- [x] 4.2 Add the "Install and restart" action streaming a detail view: "Downloading Dango X" with progress, then install, or the signature or download failure text from design.md; verify with unit tests against the fake for success, a bad signature, and a failed download
- [x] 4.3 Add the "Check for Updates" view command streaming "Checking for updates", then up to date, available with "Install and restart", failed, or the development build text; verify with unit tests against the fake for each outcome
- [x] 4.4 Add any new icon to `src/lib/Icon.svelte`, and regenerate the config schema and `docs/config.md` for the new extension; verify with `npm run check` and `cargo test`

## 5. Scheduling and tray

- [x] 5.1 In `lib.rs`, start the check task in release builds only: a short delay after setup, then every 6 hours, logging failures to `dango.log`; verify with a unit test that the scheduler is not started when `cfg!(debug_assertions)` is set
- [ ] 5.2 Add the "Install Dango X" tray item from managed state, inserted and removed on state change, which shows the launcher and starts the install flow; verify with `cargo build` and a debug run with the fake source forced to report an update

## 6. Verification

- [ ] 6.1 Windows, installed release N with the updater: publish release N+1, verify the root item and the tray item appear, confirm "Install and restart", and verify the installer runs without input and Dango restarts as N+1 with config, stored data, and autostart intact
- [ ] 6.2 macOS, installed release N with no other `Dango.app` bundle on the machine: grant Accessibility and Input Monitoring, publish release N+1, install it through "Install and restart", and verify Dango restarts as N+1 with both permissions still granted and a snippet paste working
- [ ] 6.3 macOS, same install: run Empty Trash and verify it works under the hardened runtime
- [ ] 6.4 Either platform: run "Check for Updates" offline and verify the check failure text, then online on the newest version and verify "You have the latest version"

## 7. Wrap up

- [ ] 7.1 Add to AGENTS.md: ad-hoc signing changes the macOS designated requirement each build, a real signature turns on the hardened runtime and needs the Apple Events entitlement for `osascript`, and the result of 6.2 on whether duplicate `Dango.app` bundles break Accessibility; verify the entries sit under platform traps
- [ ] 7.2 Review the diff for comments that restate the code and for scope beyond the proposal; verify `hk check`, `cargo test`, and `npm run test:browser` pass
