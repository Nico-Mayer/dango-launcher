## 1. Deliver a view with its owner

- [x] 1.1 Resolve the owning extension in `run_command` and emit
      `RenderPayload { owner, tree }` on `dango://render`; verified the tree the
      frontend receives carries the owner.
- [x] 1.2 Take `viewOwner` from the render event in `src/App.svelte`, drop the
      owner from `invoke_command`'s return, and add `RenderPayload` to
      `src/lib/types.ts`; verified in a headless run that a view delivered with
      no prior `invoke_command` - which is what a command hotkey produces - runs
      `run_action` with the right extension for Enter and for an action panel
      entry, where it previously issued no call at all.
- [x] 1.3 Verify the root path is unchanged: opening the same view from root
      search still dispatches its actions to the same extension.

## 2. Keep a failure readable

- [x] 2.1 Stop clearing `failure` in `resetToRoot`, and clear it when the user
      types, runs an action, or presses Escape; verified in a headless run that
      the message is shown on the next activation for both event orders - the
      reset arriving before the failure and after it - and that typing or
      Escape clears it.

## 3. Recognise a re-encoded image as our own

- [x] 3.1 Add `image_dimensions` to the clipboard source and give a pending
      own-write its dimensions and a timestamp; verified on macOS that a 1571
      byte PNG reads back as 5427 bytes and that the re-encoding is stable.
- [x] 3.2 Match an image own-write by dimensions within a two second window,
      keeping exact matching for text; covered by three tests - a re-encoded
      image is suppressed, the same image copied after the window is recorded,
      and an image of a different size is recorded while a write is pending.

## 4. Verify on both platforms

- [x] 4.1 On macOS, confirm the whole suite and the lints pass: 411 Rust tests,
      `cargo clippy -- -D warnings`, `npm run check`, `npm run build`.
- [ ] 4.2 On Windows, open the clipboard history by its hotkey, confirm an entry
      with Enter, and confirm one from the action panel; then make a paste fail
      and confirm the message is readable on the next activation.
- [ ] 4.3 Confirm CI is green on `windows-latest` and `macos-latest`.
