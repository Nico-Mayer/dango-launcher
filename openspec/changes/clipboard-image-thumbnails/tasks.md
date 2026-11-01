## 1. Thumbnails in the history

- [ ] 1.1 `History::record` writes `<id>.thumb.png`, fitting in 384 x 384 with `RustImageData::thumbnail`, and counts its bytes in the `bytes` column; an image that does not decode is still recorded without one. Verify with `cargo test` in `history.rs`: an image entry has a thumbnail no larger than 384 px on either side with the image's aspect ratio, and an undecodable image is recorded with none
- [ ] 1.2 `Entry` carries the thumbnail path when the file exists. Verify with a `cargo test` that an entry whose thumbnail file is missing reports none
- [ ] 1.3 `remove`, `trim`, the "file has gone" path, and the orphan sweep handle the thumbnail with its image. Verify with `cargo test`: discarding and removing an image delete both files, and `History::new` keeps a live entry's thumbnail and deletes an orphaned one

## 2. Rows show the thumbnail

- [ ] 2.1 The image row's icon is the thumbnail, falling back to the full image when there is none. Verify with `cargo test` in `extension.rs`, extending the existing "an image entry has a thumbnail" test to expect the `.thumb.png` path and adding the fallback case

## 3. Render-scoped file-image cache

- [ ] 3.1 `Icons` keeps the file images the last render used and drops the rest at the end of each render. Verify with `cargo test` in `icons.rs`: a path asked for in two renders in a row is decoded once, and a path a render did not ask for is gone after it
- [ ] 3.2 Hiding the launcher empties the cache and the list and grid models. Verify with a UI test in `src/slint_shell/tests.rs` that shows a list with a file icon, hides, and finds both the cache and the rows empty

## 4. Debug build speed

- [ ] 4.1 Add `[profile.dev.package]` `opt-level = 3` for `png`, `fdeflate`, `miniz_oxide`, and `image` in `Cargo.toml`. Verify that `hk check` and `cargo test` pass

## 5. Hand checks

- [ ] 5.1 Windows: with a scratch `DANGO_CONFIG_DIR`, copy about 55 different 4K screenshots, open Clipboard History in a debug build, then press Arrow Down quickly and type, driven by `scripts/send-keys.ps1`. The selection keeps up with no visible lag, thumbnails look sharp at 150 % scaling (`scripts/screenshot.ps1`), and the working set after leaving the history is close to idle, as in the table in #8
- [ ] 5.2 macOS: the same check on a Retina display (open until a session on the Mac)

## 6. Review

- [ ] 6.1 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context, and report its findings to the author
