## 1. Packaging

- [x] 1.1 Prove the existing minisign key works with the packager signer: sign a file with `cargo packager signer sign` using the private key and password behind `TAURI_SIGNING_PRIVATE_KEY*`, and verify the signature in a `cargo test` with `cargo-packager-updater`'s verification against the public key from `5099d00^:src-tauri/tauri.conf.json`. If it fails, generate a new pair with `cargo packager signer generate`, replace both secrets, and note it in the design.
- [x] 1.2 Pin `cargo:cargo-packager = "0.11.8"` in `mise.toml`. Verify `mise install` then `cargo packager --version` on Windows.
- [x] 1.3 Add `[package.metadata.packager]` to `Cargo.toml` per design decision 2. Verify `cargo test` still passes and `cargo packager --release --formats nsis` on Windows writes one `-setup.exe` carrying the crate version.
- [ ] 1.4 macOS packaging: verify on the Mac that `cargo packager --release --formats app,dmg` writes a `.app` with `LSUIElement` and the `com.nimayer.dango` identifier, a `.dmg`, and an `.app.tar.gz`. If no `.app.tar.gz` appears, add the tar and sign step from the design risks.

## 2. Updater source

- [x] 2.1 Add `PackagerSource` in `src/extensions/updates/source.rs` per design decision 4, with the endpoint and public key as constants. Verify with `cargo test updates::` tests that the public key decodes, the updater builds with the running version, and signature errors map to `UpdateError::Signature` while other download errors map to `UpdateError::Download`.
- [x] 2.2 Wire `PackagerSource` in `src/slint_shell/mod.rs` for release builds and keep `DisabledSource` for development builds. Verify with a `cargo test` that a debug build constructs the disabled source.
- [ ] 2.3 macOS restart after install: release the single instance lock, `open -n` the bundle, exit. Verify with a `cargo test` that the bundle path is derived from an executable path inside `Dango.app/Contents/MacOS`.

## 3. Release workflow

- [x] 3.1 Rewrite the build job in `.github/workflows/release.yml`: a matrix of `windows-latest` (`--formats nsis`) and `macos-latest` (`--formats app,dmg`), secrets mapped per design decision 3, outputs and `.sig` files uploaded to the draft release. Remove Node, npm, and tauri-action.
- [x] 3.2 Add the `manifest` job that builds `latest.json` from the uploaded signatures per design decision 5, and make `publish` depend on it.
- [x] 3.3 Restore the `v*` tag trigger and keep `workflow_dispatch` as a dry run that stops before `publish`. Verify a dispatched run on `main` is green on both platforms and leaves a draft whose `latest.json` names a `windows-x86_64` NSIS entry and a `macos-aarch64` app entry, each with a non-empty signature. Delete that draft afterwards.

## 4. Docs

- [x] 4.1 Update `AGENTS.md`: drop "The updater and the release workflow are off", add the packaging facts (packager config location, the formats per platform, `latest.json` built in CI, the macOS restart through `open -n`). Verify `rg -n "native-release-pipeline" AGENTS.md` finds nothing.
- [x] 4.2 Update the install section of `README.md` for the NSIS installer and the `.dmg`. Verify each link pattern matches an asset name from the dry run in 3.3.

## 5. Manual checks on the real OS

- [x] 5.1 Windows: uninstall the old Tauri Dango, cut a real release, and install it as the standard user without an elevation prompt. Then cut the next release and update from inside Dango: progress shows, the installer runs passively, Dango comes back as the new version with the same config, and the autostart entry points at the installed path.
- [ ] 5.2 macOS: install the `.dmg` from the same releases, pass the one-time Gatekeeper override, grant Accessibility and Input Monitoring, then update from inside Dango: Dango restarts as the new version once, without a second instance, and both grants are still in place.
