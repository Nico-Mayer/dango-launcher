# Proposal

## Why

Design system v2 adds a set of progress and status parts: a progress bar,
ring, and spinner, status dots, and skeleton rows. Dango has only the
SearchBar's sweep and a still dot in the busy toast, so busy and idle look
alike, and a loading list with nothing in it shows a blank body. Fast loads
also blink: the loading bar and "Loading…" toast flash for a single frame.
This is change 2 of the v2 stack, on top of `theme-graphite`, whose status
roles (`working`, `blocked`, `done`, `idle`) these parts use.

## What Changes

- **New atoms** in `ui/atoms/`:
  - **Progress:** a bar, a ring, or a spinner. Each is determinate (a value
    from 0 to 1) or indeterminate.
  - **Status dot:** working, needs input, done, failed, or queued, each in its
    status role.
  - **Skeleton rows:** grey placeholder rows for a list that has nothing yet.
- **The "Nothing animates" requirement is removed.** Motion is allowed where
  it shows that something is busy.
- **The busy toast shows a spinner** in the working role in place of its
  still dot. Success and failure dots stay still.
- **The SearchBar's loading bar** becomes the progress bar atom,
  indeterminate. It looks the same as today.
- **Loading marks stop blinking.** The loading bar and the busy toast appear
  only after the view has been loading for about 150 ms. Once shown, they
  stay at least about 400 ms.
- **A list that is loading with no items shows skeleton rows** in place of a
  blank body.
- **View protocol:** list items can declare two new accessory kinds, a
  `progress` ring and a `status` dot. The change is additive: every tree that
  is valid today stays valid, and the protocol stays at version 2.

## Non-goals

- No task registry, no ProgressView, no toast actions or Cancel. Those come in
  the task-registry change.
- No skeleton for grids, details, or forms. They keep today's loading marks.
- No root-search loading mark.
- No feature uses the ring or the status dot yet. AI commands adopt the
  status dot in the adopt-components change.
- No entrance or exit motion for the launcher, the panel, or the toast.

## Platforms

Windows and macOS. The change is platform-neutral: Slint atoms, the reducer,
the renderer, and the protocol types. No OS code changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `design-system`:
  - The "Nothing animates" requirement is removed.
  - The busy toast shows a spinner.
  - The small parts include progress marks, status dots, and skeleton rows.
  - The loading bar honours the minimum display time.
- `view-protocol`:
  - Accessories gain the `progress` and `status` kinds.
  - Loading marks wait before they appear and stay a minimum time.
  - An empty list that is loading shows skeleton rows.

## Impact

- `ui/atoms/`: new `progress.slint`, `skeleton.slint`. `dot.slint` takes a
  status in place of the toast style. `accessory.slint` and `gallery.slint`
  gain the new kinds.
- `ui/shell/search-bar.slint`, `action-bar.slint`, `hud.slint`, and
  `ui/views/list.slint`.
- `src/protocol.rs`: two new `Accessory` variants.
- `src/ui/`: loading-mark timing in the reducer (new effect and event).
  `src/app/` schedules the timer, as it does for toast expiry.
- `src/slint_shell/render/`: maps the new accessories and the skeleton.
- Tests in `src/ui/tests.rs` and `src/slint_shell/tests.rs`.
