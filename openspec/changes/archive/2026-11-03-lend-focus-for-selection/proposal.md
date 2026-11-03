# Proposal

## Why

AI commands like Improve Writing fail with "Select some text first" in GPUI apps
(Zed, Herdr), even with text selected. On Windows the launcher also closes
before the read, so the failure only shows on the next activation. Reading a
selection should never close the launcher without user input.

Probe results (Windows, Herdr): UI Automation sees only a bare `Zed::Window`
with no text pattern, so Dango falls back to Ctrl+C, which GPUI apps and
terminals do not treat as copy. Ctrl+Insert copies in Zed (tested by hand).

## What Changes

- Reading the selection no longer hides the launcher. On Windows Dango lends
  the foreground to the previous window while the launcher stays on screen,
  reads, copies if it must, and takes the foreground back. On macOS the panel
  stays key and the copy chord is posted to the frontmost application's
  process.
- The Windows copy fallback sends Ctrl+Insert instead of Ctrl+C. Ctrl+C is
  SIGINT in terminals and a command in modal editors; Ctrl+Insert is the
  common copy chord with no destructive meaning.
- On macOS the direct read ignores an element that belongs to Dango itself,
  and trusts an empty answer only from an element that reports a selected text
  range. Otherwise the read counts as "cannot ask" and falls back to copying.
- **BREAKING**: the config key `selection.excluded-applications` is removed. It
  existed because Ctrl+C is destructive in some editors; Ctrl+Insert removes
  that reason. No migration (alpha).
- A failed selection read is shown in the launcher right away, not carried to
  the next activation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `selection-and-paste`: reading the selection keeps the launcher on screen;
  the Windows copy chord changes; macOS empty answers need a selected text
  range; the exclusion requirement is removed.
- `launcher-shell`: reading a selection is no longer a reason for a command to
  hide the launcher.

## Platforms

Windows and macOS. Windows is built and verified in this change. macOS is
designed here, with its hand checks left open for a session on a Mac.

## Non-goals

- Herdr support. It exposes no text pattern and copies on Ctrl+Shift+C or on
  mouse selection. No general approach covers it.
- Reading the selection at every launcher activation. Rejected in the design.
- Per-application copy chords or any other per-application list.
- Changes to inserting text or pasting history entries. Those still hide the
  launcher.
- Keyword expansion's `{selection}`, which runs without a launcher.

## Impact

- `src/text/mod.rs`: `TextExchange::selection` lends instead of dismissing;
  `RefusesFallback`, `SelectionRefused` removed; new handoff step to take the
  foreground back.
- `src/platform/windows/`: `text.rs` (copy chord, lend and return),
  `mod.rs` (pause focus-loss hide while lent), `selection.rs` unchanged.
- `src/platform/macos/text.rs`: own-process check, selected-range check, copy
  posted to a pid.
- `src/app/startup.rs`, `src/app/mod.rs`, `src/platform/mod.rs`: wiring.
- `src/config/mod.rs`, `src/config/schema.rs`, `docs/config.md`: drop
  `selection` section.
- `examples/walkthrough/macos.rs`: drop the exclusion walkthrough.
