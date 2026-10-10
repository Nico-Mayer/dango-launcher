## Context

See proposal.md for motivation. What shapes the approach:

- The backend already has one notion of "development build":
  `cfg!(debug_assertions)`, used by `autostart.rs` and `latency.rs`. The
  autostart spec calls this "a debug build".
- The launcher frame is a single component, `LauncherFrame.svelte`, used by
  all three top-level branches of `App.svelte` (protocol error, pushed view,
  root search). It already draws a 1px edge from the `border-card` token.
- The footer, `LauncherFooter.svelte`, already shows a "Dango" label and is
  the only place the product name appears on screen.
- Presentation is token-driven (`tokens.css` maps `--dango-*` roles onto the
  Bits UI names) and the visual-system spec expects every surface to follow
  the shared tokens.
- The frontend mounts once, while the window is parked offscreen, and calls
  `warmup_done`. Activation only shows the window; nothing is reloaded.
- Browser fixtures mount the real `App.svelte` with `mockIPC`. Unknown
  commands fail, so any new command must be mocked there.

## Goals / Non-Goals

**Goals:**

- One definition of "development build", owned by Rust, consumed by the
  renderer as a boolean.
- Zero cost on the activation path.
- The marker is unmistakable at a glance but does not move a single pixel of
  layout, so the density scenarios in the visual-system spec keep passing.

**Non-Goals:**

- Anything listed under Non-goals in the proposal.
- Making the marker configurable or themeable beyond the one token.

## Decisions

### Which "dev" to detect

`cfg!(debug_assertions)` in Rust, exposed through one invoke command.

Alternatives considered:

- `import.meta.env.DEV` in the frontend. One line, no Rust. Rejected for two
  reasons. It means "served by the Vite dev server", which is also true for
  the browser fixtures and Playwright, so every existing presentation test
  would start rendering the marker and the release state could not be tested
  at all. It also diverges from the meaning the project already uses: a
  `tauri build --debug` artifact ignores autostart and prints latency but
  would show no marker.
- `tauri::is_dev()`. This is `!cfg!(feature = "custom-protocol")`, so it is
  true only under `tauri dev` and false for `tauri build --debug`. Same
  divergence from the existing definition, so rejected for consistency.

### How the renderer learns it

A new invoke command, `is_development_build`, returning a bool. `App.svelte`
calls it once in `onMount`, next to `warmup_done`, and keeps the answer in
state. The command returns a constant, so it adds nothing measurable to
warm-up and nothing at all to activation.

Alternatives considered:

- Returning the flag from `warmup_done`. Saves a command but couples a
  notification with a query. Rejected: a dedicated command reads better in the
  fixture mock and costs one line.
- An initialization script that sets a global on `window`. Rejected: the
  window is declared in `tauri.conf.json`, and a script would bypass the
  typed invoke path every other piece of state uses.

### Where the marker is drawn

`App.svelte` passes `development` as a prop to `LauncherFrame`, which sets a
`data-development` attribute on the `<main>` element. CSS on that attribute
swaps the edge colour to a new token, `--dango-dev-edge`, and adds an inset
box shadow of one edge width in the same colour. The result is a 2px coloured
edge with no layout shift, because the border stays 1px and the shadow takes
no space.

`LauncherFooter` always renders a "dev" badge after the "Dango" label and
shows it only when an ancestor carries `data-development`, through Tailwind's
`in-data-development:` variant. The footer takes no prop: pushed views render
their own footer inside `ProtocolView`, and reading the frame's attribute
covers every footer without threading a value through the view protocol. The
badge uses the existing amber tint tokens (`bg-tint-amber`,
`text-tint-amber-foreground`) and the keycap radius. It is text, so it
satisfies the visual-system rule that state is not conveyed by colour alone.

The token resolves to a new palette entry, `--dango-palette-dev-edge`, a
saturated amber in both themes. The amber tint foreground was tried first and
rejected after a screenshot: at the tint's lightness it reads as a pale or
near-black border, not as a warning. Amber is already the project's "caution"
tint, so the hue stays in the family.

Alternatives considered:

- Changing the search placeholder or the window title. Rejected: invisible on
  the protocol error screen and in pushed views.
- A full-width banner. Rejected: it changes density and competes with the
  status banner and the busy sweep.

### Tray tooltip

`TrayIconBuilder::tooltip` with "Dango (development build)" or "Dango",
chosen by the same `cfg!(debug_assertions)`. One expression at the build
site, no new abstraction.

### Fixtures and tests

`tests/fixtures/main.ts` mocks `is_development_build` and answers from a
`dev=1` query parameter, default false. A new browser test opens the root
scene in both states and asserts the attribute, the badge, and that row and
footer heights are unchanged. It also checks the attribute on a pushed view
scene, which exercises the same frame component on a second surface.

## Risks / Trade-offs

- [The marker is painted by the webview, so it could lag the window on first
  paint] → The answer arrives during warm-up, before the window is ever
  shown, so by the first activation the marker is already painted.
- [Amber edge clashes with the focus or selection colour] → The edge is on the
  outermost frame, where no focus ring or selection is drawn. The token can
  be overridden if a different colour is wanted later.
- [A fixture forgets to mock the command] → The fixture harness fails loudly
  on unknown commands, so the test suite catches it immediately.
