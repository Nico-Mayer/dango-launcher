## 1. Baseline and repeatable frontend fixtures

- [ ] 1.1 Add a small development-only browser fixture setup with stubbed Tauri invocation/events for root results, populated/empty lists, streaming details, forms, failures, and long action lists; verify fixtures load without native commands or credentials and are excluded from the production entry.
- [ ] 1.2 Add repeatable browser checks for existing non-wrapping selection, resting-pointer suppression, first-row scrolling, delayed result identity, typed-query focus, form Enter versus modified Enter, and carried failures; record baseline screenshots at 720x400 and run `npm run check` and `npm run build` successfully.

## 2. Theme-ready token foundation

- [ ] 2.1 Introduce foundation, built-in palette, and semantic/component token layers under `src/lib/theme/`, imported by `src/app.css`, initially matching existing visuals through compatibility aliases; verify search/result/footer dimensions remain 64/56/40 CSS pixels and check/build pass.
- [ ] 2.2 Separate selection, hover, focus, control, keycap, error, busy, and text roles, including existing tint pairs; verify development overrides change all corresponding surfaces, including portaled content, without changing unrelated states.
- [ ] 2.3 Tokenize recurring typography, spacing, radii, edges, elevation, icon sizes, motion, and layer values while retaining transparent document corners and synchronous dark startup; verify both built-in palettes render in fixtures and no theme file loading, picker, or configuration behavior was added.

## 3. Shared presentation without orchestration changes

- [ ] 3.1 Extract thin Bits Command presentation wrappers under `src/lib/ui/` and reuse them in root and pushed lists; verify refs, bindings, stable IDs, group insets, first-row scrolling, backend ranking, and all baseline navigation checks remain intact.
- [ ] 3.2 Extract launcher frame/footer composition under `src/lib/launcher/` plus shared shortcut, button, empty/error, and status presentation; verify root, detail, form, and protocol-error states share geometry while carried failures and invocation ownership remain unchanged.
- [ ] 3.3 Extract native text/password/textarea field composition and a Bits Checkbox wrapper with associated labels/help/errors; verify first-field focus for every field kind, string-valued checkbox submission, unchanged initial values, editable templates, and multiline submission shortcuts.
- [ ] 3.4 Migrate `ResultRow.svelte` and shared icon presentation to size/radius/text tokens without replacing the existing icon provider; verify named tints, highlighted matches, contained application icons/favicons, framed clipboard previews, and unknown-icon fallback in fixture screenshots.
- [ ] 3.5 Run `npm run check`, `npm run build`, baseline browser checks, and Svelte component analysis for touched files; verify this extraction stage is runnable before action-panel behavior changes.

## 4. Bits action panel migration

- [ ] 4.1 Prove Popover plus a separate portaled Command scope in the action fixture, without a new search field; verify first-action selection, non-wrapping arrows, pointer-independent selection, selected-option accessibility relationship, and isolation from the parent Command DOM and event handlers.
- [ ] 4.2 Replace `ActionPanel.svelte` custom navigation/positioning with the verified composition and connect the footer Actions trigger plus existing keyboard chords; verify root/list/form/detail origins, long-action scrolling, empty-action handling, exactly-once invocation, and parent selection preservation.
- [ ] 4.3 Wire primitive focus hooks and narrow launcher-specific Escape/shortcut adapters, including exit-presence ownership; verify immediate and repeated Escape/Enter, pointer activation, close/reopen, outside interaction, and focus return without triggering the parent action or unintended launcher dismissal.
- [ ] 4.4 Run check/build and action-panel integration checks with motion disabled; verify no custom Arrow/Enter navigation loop remains and the app is runnable before enabling transitions.

## 5. Neutral visual refinement and purposeful motion

- [ ] 5.1 Apply the neutral graphite direction, restrained blue selection/focus, coherent surface depth, typography weights, and optical spacing through shared tokens/components; verify 56px rows/64px headers, readable metadata, distinct hover/selection/focus, and unchanged extension identities in before/after screenshots.
- [ ] 5.2 Add token-driven hover feedback and Bits-managed action-panel entrance/exit transitions; verify each overlay transition is at most 180ms, activation and keyboard selection remain immediate, keyboard-cleared hover does not fade slowly, and no result staggering or view-replacement entrance is introduced.
- [ ] 5.3 Add one shared busy edge sweep plus a stable status presentation driven only by `workingTitle` and list/detail `loading`; verify empty loading lists, populated refreshes, streaming details, false-to-true/true-to-false transitions, failure, abandonment, and hide without clearing content, shifting the result viewport, or inventing root-search progress.
- [ ] 5.4 Expose busy semantics and a stable polite status region, preserve the exact copy listed in design.md, and remove the generic pending spinner in favor of the shared treatment; verify no numerical progress claim, duplicated empty-loading label, per-chunk announcement, or extension-specific content parsing.
- [ ] 5.5 Add reduced-motion and hidden-surface handling; verify live preference changes stop sweeps/pulses/transforms, retain static status, complete overlay dismissal without animation-dependent deadlocks, and leave no busy loop after reset/unmount or document hiding.
- [ ] 5.6 Verify fixture contrast, long-content layout, and token overrides in both built-in palettes at 720x400 and 480x300; require enabled normal text at least 4.5:1, focus indicators at least 3:1, reachable footer controls, bounded/scrolled action content, and no horizontal overflow.

## 6. Cross-cutting regression and native verification

- [ ] 6.1 Run all frontend browser regressions, `npm run check`, `npm run build`, and Svelte analysis on changed components; verify thirty same-view replacements per second preserve focus/selection without replayed entrances, and inspect the final diff for duplicated visual literals, unnecessary comments, protocol changes, or theme-loader scope creep.
- [ ] 6.2 Verify the Windows native build visually and interactively: Ctrl+K, panel Escape isolation/focus return, forms, long actions, icons/favicons, loading/failure states, 200 percent display scaling, and transparent edges; record screenshots and outcomes rather than treating browser checks as native evidence.
- [ ] 6.3 Verify the macOS native build visually and interactively: Cmd+K and Ctrl+K, panel Escape isolation/focus return, forms, long actions, icons/favicons, loading/failure states, Retina/scaled rendering, and transparent edges; record screenshots and outcomes separately from Windows.
- [ ] 6.4 Verify Windows accessibility using NVDA and the operating system motion preference: named controls, focused action selection, field error associations, non-repeating busy announcements, live reduced-motion changes, and static loading feedback; record any limitation and leave unverified cases open.
- [ ] 6.5 Verify macOS accessibility using VoiceOver and Reduce Motion: named controls, focused action selection, field error associations, non-repeating busy announcements, live preference changes, and static loading feedback; record any limitation and leave unverified cases open.
- [ ] 6.6 Build Windows release through `npx tauri build --no-bundle` and measure first/repeat activation with `DANGO_MEASURE=1`; verify each measured activation is within 80ms and inspect streaming/hidden-window traces for animation-induced dropped frames or surviving loops, recording measurements and environment.
- [ ] 6.7 Build macOS release through `npx tauri build --no-bundle` and measure first/repeat activation with `DANGO_MEASURE=1`; verify each measured activation is within 80ms and inspect streaming/hidden-window traces for animation-induced dropped frames or surviving loops, recording measurements and environment.
