# Browser test inventory

The launcher state machine lives in Rust (`src-tauri/src/ui/`), and the tests
of its rules are in `src-tauri/src/ui/tests.rs`. Each browser test left here is
in one of two buckets:

- **dies**: webview presentation. Stays, fed snapshots, and is deleted with the
  webview in `replace-webview-with-slint`.
- **slint**: stays, then is ported to a Slint test in
  `replace-webview-with-slint`.

Titles are the `test(` templates as written in the file.

## `ActionPanel.integration.spec.ts`

| Test | Bucket |
| --- | --- |
| `ActionPanel: long actions scroll inside compact portal in both palettes` | dies |
| `ActionPanel: ${scene} outside ${press} preserves parent` | dies |
| `ActionPanel: ${scene} immediate ${key} and held repeats stay isolated` | dies |
| `ActionPanel: trigger keyboard activation does not confirm parent` | dies |
| `ActionPanel: ${closingKey} closed-presence blocks fresh keys and clicks` | dies |
| `ActionPanel: actual portal inherits live typography, radius and surface overrides` | dies |

## `BusyStatus.spec.ts`

| Test | Bucket |
| --- | --- |
| `BusyStatus: ${scene} preserves content, geometry and declared lifecycle` | dies |
| `BusyStatus: streaming keeps one sweep/status node and does not announce chunks` | dies |
| `BusyStatus: real surfaces inherit live independent busy token overrides` | dies |
| `BusyStatus: live ${change} stops loops and panel exit releases ownership` | dies |
| `BusyStatus: unmount removes busy loop and visibility listener` | dies |
| `BusyStatus: live reduced motion removes pointer feedback while keeping selection` | dies |

## `development-indicator.spec.ts`

| Test | Bucket |
| --- | --- |
| `development indicator: ${scene} marks a development build` | slint |
| `development indicator: ${scene} shows nothing in a release build` | slint |
| `development indicator: protocol error screen keeps the edge` | slint |
| `development indicator: edge colour follows the token in both palettes` | dies |

## `fixtures.spec.ts`

| Test | Bucket |
| --- | --- |
| `fixture ${name} loads without native IPC` | dies |

## `navigation.spec.ts`

| Test | Bucket |
| --- | --- |
| `${scene}: non-wrapping selection and first-row scrolling` | dies |
| `${scene}: pointer hover never selects and keyboard suppresses resting hover` | slint |
| `${scene}: clicking keeps typed-query focus` | slint |
| `root: the first row of a later group is scrolled fully into view` | slint |

The non-wrapping rule of the first test is also tested in `cargo test`. The
test stays for its scrolling half.

## `presentation.spec.ts`

| Test | Bucket |
| --- | --- |
| `presentation: every view fills the window with the footer at the bottom` | dies |
| `FormFields: template help and editable errors stay associated` | dies |

## `refinement.spec.ts`

| Test | Bucket |
| --- | --- |
| `refinement: motion keeps navigation immediate and exit owns fresh input` | dies |

## `ResultRow.spec.ts`

| Test | Bucket |
| --- | --- |
| `ResultRow: ${palette} identities, matches, previews and fallback` | dies |
| `ResultRow: icon, radius, edge and typography tokens reach each presentation` | dies |

## `theme.spec.ts`

| Test | Bucket |
| --- | --- |
| `theme: independent states reach ${scene}, actions, and portal` | dies |
| `theme: controls, errors, pending status and tints resolve independent roles` | dies |
| `theme: text hierarchy and raised surface inherit from document root` | dies |
| `theme: both built-in palettes render on a transparent window` | dies |
| `theme: foundation overrides reach existing consumers and portal` | dies |

## `visual-accessibility.spec.ts`

| Test | Bucket |
| --- | --- |
| `visual: ${palette} ${width} long content, contrast, focus and bounded actions` | dies |
| `visual: ${palette} fields/errors/trigger focus contrast and live state overrides` | dies |
