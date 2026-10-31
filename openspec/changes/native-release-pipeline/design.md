## Context

`replace-webview-with-slint` left Dango without packaging. What is still in
place:

- `UpdateSource` (`src/extensions/updates/source.rs`): check, download with
  progress, install. Production wires `DisabledSource`; the updates extension,
  its views, its strings, and its tests are unchanged and use a fake.
- The Tauri implementation, `PluginSource`, is in git history at
  `5099d00^:src-tauri/src/extensions/updates/source.rs`. It kept the found
  update between check and install and mapped minisign errors to
  `UpdateError::Signature`.
- The updater endpoint and public key are in
  `5099d00^:src-tauri/tauri.conf.json` (`plugins.updater`). The endpoint is
  `https://github.com/Nico-Mayer/dango-launcher/releases/latest/download/latest.json`.
- Repository secrets: `TAURI_SIGNING_PRIVATE_KEY`,
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `APPLE_CERTIFICATE`,
  `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`.
- `Info.plist` (with `LSUIElement`), `Entitlements.plist`, `icons/icon.icns`,
  `icons/icon.ico`.
- `release.yml` has a working changelog job (git-cliff, draft release) and a
  publish job. Only the build job is Tauri-specific.

## Goals / Non-Goals

**Goals:**

- A `v*` tag yields a published release with an NSIS installer, a signed macOS
  `.dmg`, both updater artifacts with signatures, and `latest.json`.
- A release build finds, downloads, verifies, installs, and restarts into a
  newer release, as `app-updates` specifies.
- The same minisign key and Apple identity as the Tauri releases.

**Non-Goals:**

- Windows code signing, notarization, Intel macOS, delta updates.
- Any change to the updates extension, its views, or its strings.
- Packaging from a developer machine as part of the normal workflow. It works
  locally, but releases come from CI.

## Decisions

### 1. cargo-packager and cargo-packager-updater

Packaging uses `cargo-packager` and updates use `cargo-packager-updater`, both
from CrabNebula. The packager is forked from tauri-bundler, and the updater
from the Tauri updater plugin. So the artifacts, the minisign signatures, and
the update flow are the ones the Tauri releases had.

Checked in the updater source:

- `Updater::check` returns an `Update`.
- `Update::download_extended` reports each chunk and the total, and verifies
  the minisign signature against the configured public key before it returns
  the bytes.
- `Update::install(bytes)` is a separate step, which matches `UpdateSource`.
- On Windows, `install` starts the NSIS installer with `/P /R` (passive mode,
  relaunch the app when done) and calls `process::exit(0)`.
- On macOS, `install` unpacks the `.app.tar.gz`, swaps it in place of the
  running bundle, and returns. Restarting is the caller's job.

Alternatives rejected:

- **Velopack**: active, delta updates, per-user install. It needs the .NET
  `vpk` tool in CI, and it verifies packages by hash over HTTPS rather than a
  signature against a key built into the app. That would weaken the
  `app-updates` requirement that an update is refused when its signature does
  not match.
- **self_update with a zip**: no installer, no `.app` or `.dmg`, and no Start
  menu entry. Dango would need its own install layout.
- **cargo-dist**: aimed at CLI tools; no `.app` bundle.

The risk is cadence: the last packager release is 0.11.8 (November 2025),
though the repository is still active. Both tools are pinned, so a stall costs
nothing until a platform change breaks them.

### 2. Packager config in `Cargo.toml`

`[package.metadata.packager]` in `Cargo.toml`, not a separate
`Packager.toml`, so the version, name, and packaging live in one file:

- `product-name = "Dango"`, `identifier = "com.nimayer.dango"`
- Windows: `formats = ["nsis"]`, NSIS `install-mode = "currentUser"`, icon
  `icons/icon.ico`
- macOS: `formats = ["app", "dmg"]`, `info-plist-path = "Info.plist"`,
  `entitlements = "Entitlements.plist"`, icon `icons/icon.icns`
- `before-packaging-command = "cargo build --release"`

The formats are passed per platform on the command line
(`cargo packager --release --formats nsis` or `app,dmg`), since one config
covers both.

### 3. Keys and secrets

- **Update signing:** cargo-packager signs every output when
  `CARGO_PACKAGER_SIGN_PRIVATE_KEY` and `..._PASSWORD` are set. Its key format
  is the same base64 minisign box Tauri uses. The workflow maps the existing
  `TAURI_SIGNING_PRIVATE_KEY*` secrets to those names; the secrets are not
  renamed.
- **Public key:** restored from git history into a constant in `source.rs`,
  next to the endpoint. It is public, so it is checked in.
- **macOS certificate:** cargo-packager imports `APPLE_CERTIFICATE` and
  `APPLE_CERTIFICATE_PASSWORD` into a temporary keychain on its own. It reads
  the signing identity from its config, not from the environment, and
  `--config` replaces the `Cargo.toml` table instead of merging into it. So
  the macOS job writes `signingIdentity` from `APPLE_SIGNING_IDENTITY` into
  the `Cargo.toml` of its checkout before packaging. Nothing secret is
  committed.

The first task proves the old key signs and verifies with the packager tools
before anything else is built on it. If it does not, a new key pair replaces
it; nothing installed depends on the old key, since the only install is the
author's and gets reinstalled by hand.

### 4. `PackagerSource`

A new `PackagerSource` implements `UpdateSource`, ported from the old
`PluginSource`:

- It builds an `Updater` with the endpoint, the public key, and the running
  version (`CARGO_PKG_VERSION`). The default Windows install mode is passive,
  which the "Installing on Windows" scenario needs.
- The updater is blocking, so `check` and `download` run on
  `tokio::task::spawn_blocking`. Progress is forwarded from the chunk callback.
- The found `Update` is kept between check and install, as before.
- Signature failures map to `UpdateError::Signature`, the rest of a download
  to `UpdateError::Download`. The packager's error variants are matched the
  same way the Tauri ones were.
- A development build keeps `DisabledSource`. The extension already refuses to
  check in a development build; this keeps the dev binary from holding a
  network-capable source at all.

**Restart on macOS:** after `install` returns, Dango releases its single
instance lock, starts the bundle again with `open -n <bundle path>`, and
exits. Releasing the lock first matters: otherwise the new process finds the
old one, hands it a "show" message, and exits. The bundle path is the
updater's own `Update::extract_path`, which it derives from the executable
inside `Contents/MacOS` and installs into, so Dango does not derive it again. Windows needs nothing, since
the installer relaunches Dango.

Alternatives: re-executing the binary directly would start it outside its
bundle, and macOS would not apply its bundle identity; `open` is the system's
own launcher for a bundle.

### 5. `latest.json` is built in CI

cargo-packager writes `.sig` files but no manifest; tauri-action used to write
it. A `manifest` job after both builds:

1. Downloads `*.sig` assets from the draft release.
2. Writes `latest.json` with `version`, `pub_date`, `notes` (the changelog
   section), and `platforms`:
   - `windows-x86_64`: the NSIS `-setup.exe`, its signature, `format: nsis`
   - `macos-aarch64`: the `.app.tar.gz`, its signature, `format: app`
3. Uploads it to the release.

The platform keys are the updater's (`macos-…`, not Tauri's `darwin-…`).
The script is a few `jq` lines in the workflow. A `scripts/*.mjs` was
considered, but the job already runs on Ubuntu with `gh` and `jq`.

GitHub's `releases/latest` never points at a draft or a pre-release, which is
what `app-updates` asks for, so the endpoint needs no filtering.

### 6. Workflow shape

```
tag v* --> changelog (draft release, notes)
              |
              +--> build windows-latest  --> nsis .exe + .sig     --+
              +--> build macos-latest    --> .dmg, .app.tar.gz+.sig -+
                                                                    v
                                                    manifest (latest.json)
                                                                    |
                                                                    v
                                                      publish (draft=false)
```

Each build job installs Rust, restores the cache, installs `cargo-packager` at
the version pinned in `mise.toml`, runs it, and uploads its outputs with
`gh release upload`. If any job fails, publish never runs and the release
stays a draft.

`cargo-packager` is pinned in `mise.toml` (`cargo:cargo-packager`) so a local
`cargo packager` matches CI.

### 7. Install layout and old installs

The NSIS installer installs to `%LOCALAPPDATA%\Dango` per user, with a Start
menu entry and an uninstaller. Config and data stay where `config/location.rs`
puts them, independent of the install path, so an update keeps them.

The old Tauri install is removed by hand before the first new install. Its
autostart entry used the same `Dango` name, so the first start with
`launcher.autostart` on rewrites it to the new path.

### 8. Contracts

Neither the extension manifest nor the view protocol changes. No string is
added or changed: every updater string already exists in `UpdateError` and the
updates extension.

## Risks / Trade-offs

- **Old key incompatible with the packager signer** -> task 1.1 checks it
  first, and the fallback is a new key pair.
- **`.app.tar.gz` not produced by the packager on its own** -> the updater
  tests expect it next to the `.app`. If the packager does not create it, the
  macOS job tars the `.app` and signs it with `cargo packager signer sign`.
- **Signing identity override** -> resolved: there is no command-line
  override, so the job writes `signingIdentity` into `Cargo.toml` (decision 3).
- **Restart race on macOS** -> releasing the lock before `open -n` is the
  fix; checked by hand in the update test.
- **Packager release cadence** -> pinned version; revisit only if a macOS or
  Windows change breaks it.
- **SmartScreen** on each Windows install and update download from the browser.
  Updates through the app do not go through the browser, so they are not
  blocked by mark-of-the-web.
