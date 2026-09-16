## Context

See proposal.md - Why. Source inspection found a small frontend, not a framework that needs replacing:

- `App.svelte` owns invocation events, root search, the view stack, failure persistence, and focus recovery. Root search already uses Bits `Command` with filtering and pointer selection disabled.
- `ProtocolView.svelte` repeats the root search input/list/row styling. Its loading branch only handles empty lists; detail rendering ignores `DetailView.loading`.
- `ActionPanel.svelte` is a positioned button list with a capture-phase window keyboard listener. It owns its selection by array index and has no primitive-managed focus or overflow lifecycle.
- `FormFields.svelte` uses native controls and deliberately keys form state on a replacement tree. `ResultRow.svelte` already centralizes icon rendering, match emphasis, and extension tints.
- `app.css` has useful light/dark color tokens but only two radius aliases; recurring sizes and styles live in component utility strings. `index.html` fixes the app to dark mode.
- The native window is 720 by 400, transparent, non-resizable, with native shadow disabled. Its content fills the webview, leaving no exterior margin for a CSS shadow.
- Existing list-navigation requirements include non-wrapping arrows, selection unaffected by hover, immediate hover suppression on keyboard use, and prompt focus retention. These are invariants, not redesign opportunities.
- The AI producer already emits loading detail trees. Its initial content is "Working…". No stats surface exists, and markdown formatting is explicitly deferred in the roadmap.
- Frontend scripts provide type checking and builds, but no dedicated frontend test runner is declared. Existing browser inspection artifacts do not constitute a regression suite.

## Goals / Non-goals

**Goals:** Establish a small reusable presentation layer, preserve event/state ownership, and make visual feedback accurately follow existing state. Centralize enough values that future themes do not require feature edits.

**Non-goals:** Do not create a public theme API, a second UI framework, arbitrary component skinning, or speculative wrappers for unused controls. Do not change the backend, generated protocol types, view-stack semantics, markdown rendering, or native window geometry.

## Decisions

### 1. Preserve the composition; refine its visual hierarchy

Confirmed direction: neutral and precise, with existing density retained. Use graphite surfaces, restrained blue focus and selection, and the existing extension tint families. Search remains the focal point; selected rows lead over supporting metadata and footer hints.

Domain references are command recall, keyboard shortcuts, clipboard content, execution, and returning to the previous app. The signature is a consistent rhythm of tinted identity tiles and a quiet activity edge, not a generic dashboard grid.

Starting values below are implementation defaults, not separately confirmed palette swatches. Tune only within this direction after native visual review:

| Property | Starting treatment |
| --- | --- |
| Canvas / inset / raised dark surfaces | Neutral graphite with small lightness steps; inset fields slightly darker, overlays lighter |
| Text | Primary, secondary, tertiary, disabled roles; enabled metadata must still meet contrast requirements |
| Selection | Blue-tinted fill plus a quiet inset edge; stronger than hover without changing row geometry |
| Focus | Separate high-contrast ring on discrete controls (fields, checkbox, footer trigger), not the same treatment as list selection. Whole surfaces never carry a ring: the search prompt holds focus for the whole session under the list-navigation requirements and its caret marks focus, and the action panel's command scope conveys focus through its selected row via `aria-activedescendant`. A ring framing either surface collided with the rounded shell and overlay corners in native review |
| Search / result / footer | Preserve 64px / 56px / 40px at default text scale |
| Type | Tokenized local system sans stack for native legibility; 24px search, 14px result/body, 12px metadata; 400/500/600 weights |
| Spacing | 4px base rhythm with explicit 2px micro-spacing where existing icon/keycap alignment requires it |
| Radius | Preserve a 14px shell starting point; coherent smaller row/control/overlay values with nested radii adjusted to padding |
| Depth | Surface hierarchy and subtle inset edges; restrained shadow only for floating content inside the native window |
| Icons | Preserve 32px identity tiles and 20px named result glyphs; expose shared size roles for other icon uses |

Reject a new downloaded typeface: it introduces loading and platform variation without a confirmed need. Reject translucency, background blur, large gradients, or bigger cards as substitutes for hierarchy. No native exterior shadow change belongs here.

### 2. Three token layers, all resolved through CSS variables

Use `src/lib/theme/foundations.css`, `themes.css`, and `tokens.css`, imported by `src/app.css`. Keep root transparency and document-level resets in `app.css`.

```text
Foundation scales
       |
       v
Semantic roles <--- built-in dark/light values
       |                    ^
       v                    |
Component aliases     future validated theme overrides
       |
       v
Shared components ---> launcher and protocol views
```

- Foundation values: spacing, font family/size/weight/line-height/tracking, radii, edge widths, icon sizes, shadow scales, layer order, duration, easing, and motion distances.
- Semantic roles: shell/canvas, inset control, raised overlay, primary/secondary/tertiary/disabled text, subtle/emphasized border, hover, selected fill/edge/text, focus, keycap, error, busy, and existing tint pairs.
- Component aliases: search height, result height, footer height, row inset, field padding, shell radius, and overlay width/viewport margin. Add aliases only where real consumers need independent tuning.

Use a consistent `--dango-*` namespace. For example, `--dango-selection-bg` and `--dango-focus-ring` must be independent. Existing names can temporarily alias the new roles during migration. Map reusable values into Tailwind v4 through `@theme inline`; keep runtime theme overrides as CSS custom properties, never dynamically generated utility names.

Feature components retain structural utilities such as flex/grid/min-width/overflow. Repeated visual values belong in shared components or named roles, not copied literal utility strings. Zero, percentages, and truly unique layout relationships need not each become a token. This is not a ban on all numeric CSS.

Put theme values on the document root so portaled content inherits them. Preserve dark startup with no asynchronous theme resolution. Keep the existing light palette complete and testable through development overrides; do not add automatic mode detection or user-facing switching.

Reject one flat bag of tokens: it couples independent states. Reject exhaustive per-component palettes: it makes themes expensive to maintain. Reject a runtime token engine or schema package: CSS already provides the necessary inheritance.

### 3. Future theme mapping is a boundary, not an implementation

An eventual theme file can map typed semantic fields onto root variables. Illustrative mapping only, not a shipped schema:

| Possible future field | Internal role |
| --- | --- |
| `colors.surface.canvas` | `--dango-surface-canvas` |
| `colors.selection.background` | `--dango-selection-bg` |
| `colors.focus.ring` | `--dango-focus-ring` |
| `typography.family` | `--dango-font-sans` |
| `radii.control` | `--dango-radius-control` |
| `motion.duration.fast` | `--dango-duration-fast` |

A future loader would supply metadata/version, choose a built-in base, validate bounded values, inherit omitted fields, reject arbitrary CSS/URLs, and retain the last valid theme on error. That requires a separate proposal. This change supplies neither a JSON file nor loader, validation, watcher, persistence, schema version commitment, or selector UI.

Native window dimensions remain separate from presentation density. Reduced-motion preferences take precedence over theme motion values. Internal token names remain refactorable until a public theme contract is deliberately introduced.

### 4. Thin shared components, not new behavior ownership

Create `src/lib/ui/` for reusable primitives and `src/lib/launcher/` for launcher-specific frame/footer composition. Keep existing feature adapters where moving them adds no value.

| Surface | Foundation and shared ownership |
| --- | --- |
| Root and pushed search lists | Bits `Command.Root`, `Input`, `List`, `Viewport`, `Item`, `Group`, `GroupHeading`, `GroupItems`; shared input/item/list styling |
| Action panel | Bits `Popover.Root`, `Trigger`, `Portal`, `Content` containing a separate Bits `Command` action list |
| Text/password fields and textarea | Native `input`/`textarea` inside shared field composition; Bits `Label` where appropriate |
| Boolean form field | Bits `Checkbox.Root` with associated label; retain existing string-valued protocol conversion at the feature boundary |
| Footer action trigger | Bits `Popover.Trigger`, styled through a shared button treatment and existing shortcut keycap |
| Frame, keycaps, row content, empty/error presentation | Semantic HTML; no interactive behavior to reproduce |
| Busy status | Native status region and `aria-busy` on the affected content; decorative sweep is CSS, not an interaction primitive |
| Detail and protocol-error views | Existing semantic content inside shared frame/status styling; do not turn them into new dialogs |

Reuse `ResultRow.svelte`, `Icon.svelte`, and tint mapping. Preserve favicon containment, clipboard preview cropping/frame treatment, unknown-icon fallback, and match emphasis.

Forward Bits props, snippets, event hooks, and bindable refs. Preserve DOM roles and attributes supplied by Bits. Keep snippets and wrapper markup near official examples. The frontend's backend calls and invocation state do not move into generic UI components.

Dialogs, tooltips, selects, and tabs should use their corresponding Bits primitives when actual features introduce them, but no unused wrappers ship now. Reject broad component-library scaffolding and a wholesale move of application orchestration.

### 5. Action panel uses Popover plus a separate Command scope

Bits `Command` already supplies the desired non-wrapping keyboard selection with pointer selection disabled. Use stable action IDs, `shouldFilter={false}`, `disablePointerSelection`, `vimBindings={false}`, and non-looping navigation. No additional action-search field is introduced.

Bits `Popover` supplies positioning, available-size constraints, outside interaction, focus trapping, and Escape lifecycle. Anchor it to the footer's existing Actions affordance, making that affordance a real trigger. Match current compact action-row density rather than forcing 56px result-row sizing onto menu actions.

Portal the content outside the parent command's DOM subtree so its selectors and bubbling navigation cannot reach nested actions. Keep styling inherited from the document root. Clamp width to available space and constrain height with an internal scroller. Focus the inner command scope on opening; explicitly expose its selected option relationship to assistive technology when no command input exists. Verify this on both screen readers rather than assuming styling alone provides accessibility.

Use Popover's autofocus hooks to return focus to the originating search input or form control, or the detail view's focusable content region. The trigger is not always the appropriate return target for a keyboard-first launcher.

Remove the custom capture-phase Arrow/Enter implementation. Retain only launcher-specific adapters: opening by platform chord, preventing panel Escape from reaching the launcher's pop/hide handler, pointer-vs-keyboard hover presentation, and returning focus to the typing context. Popover lifecycle ownership and parent keyboard gating must cover exit presence, not only the boolean open state. Do not keep an outgoing panel actionable or allow one Enter to run both panel and parent actions.

The current first-row scroll correction and user-gesture selection guard in root/pushed lists address observed primitive integration behavior. Preserve them during extraction; remove only if a targeted regression test proves the installed primitive handles the case without them. Do not expand them into replacement navigation.

Native macOS review found the row after a group heading selected but clipped below the scroller. Bits 2.19.2 wraps every item in a `display: contents` node that repeats the item's `data-value`; its "first item of a group" branch compares against that wrapper, so it matches every row and scrolls only the group heading, or nothing in an ungrouped list. The per-view first-row `scrollTop` corrections are therefore replaced by one correction in the shared list wrapper: when `data-selected` moves, the selected row is scrolled `nearest` inside the list, and the list's first row resets the scroller to the top so its heading stays visible. Bits still owns navigation and its own heading scroll; this only completes the scroll it skips.

Alternatives: keep the custom panel, or use a menu primitive by default. Keeping it retains duplicated selection and focus work. Menu behavior is not assumed interchangeable with Dango's explicit hover-independent selected-row contract; Command already exposes the required controls. The selected composition still needs a narrow integration test for focus and nested event isolation before migration proceeds.

#### Installed primitive integration (stage 4)

Bits UI 2.19.2 handles Escape at document bubble, prevents the original event, and supplies a cloned event to `onEscapeKeydown`. Content therefore allows Escape to bubble; launcher handlers honor `defaultPrevented`. Other panel keys stay inside the portaled Command scope. A small capture guard consumes input only during closed-but-present content and repeats of the closing key until its keyup. It does not select or invoke actions. `onOpenChangeComplete(false)` releases exit ownership, including zero-duration dismissal; disabled outgoing items cannot reactivate.

Bits' outside-dismiss hook runs after pointerdown (10ms deferred), too late to consume a parent row's click reliably. The launcher records an outside press, prevents its mousedown focus transfer, and consumes its resulting click even after exit has completed. Pointerdown still reaches Bits unchanged: the primitive alone decides whether to dismiss. Content/trigger presses are excluded. This adapter preserves the launcher's stronger parent-selection contract without replacing outside-dismiss behavior.

Keep the pointer-hover marker on the inner Command, not Popover.Content. In the installed primitive, changing Content props during pointer interaction can recycle its ref and dismissible-layer listeners, cancelling the deferred outside callback. The production outside-press regression covers that seam. Focus hooks capture the originating typing control and restore it immediately on close, falling back to the current view control if the original disconnected. Pushed footer presentation lives beside its ProtocolView action owner; App still supplies carried failures through a snippet and owns invocation state.

#### Same-task replacement guard (stage 5)

A same-task replacement after ArrowDown reproduced a selection reset in both root and pushed lists (6/6 failures). Bits calls `onValueChange` synchronously during input, but its deferred item re-sort also writes the first item. The previous timer-wide gesture flag admitted both writes, and Svelte binding propagation could not distinguish them. Selection now accepts the synchronous callback only while the captured input event is dispatching (`eventPhase !== NONE`); function bindings remain controlled by the derived identity and ignore deferred writes. The action Command uses the same guard because unchanged actions re-register when delayed parent results arrive. Missing IDs still fall back to the first available item. Closed action scopes expose an empty controlled value because all outgoing options are disabled; retaining a selected ID there made Bits repeatedly reconcile an unavailable value during reset. Bits still owns all navigation, confirmation and scrolling; no scheduling delay or replacement navigation was added.

### 6. Motion follows identity and lifecycle, not render frequency

Use CSS state transitions and existing Svelte/Bits capabilities. Bits supports CSS entrance/exit presence and a delegated child snippet plus `forceMount` for Svelte transitions; prefer its normal CSS presence path here. Do not wrap content in a parent `{#if panelOpen}` that destroys it before Bits can finish its exit.

Starting motion tokens:

| Use | Duration | Treatment |
| --- | --- | --- |
| Launcher activation, keyboard selection, keyboard-cleared hover | 0ms | Immediate |
| Pointer hover and button color feedback | 100ms | Named color/opacity properties only |
| Panel entrance | 140ms | Opacity plus small scale from 0.98, anchored to the trigger |
| Panel exit | 90ms | Opacity; no delayed keyboard handoff |
| Busy sweep loop | 1400ms | Narrow translated gradient inside a clipped edge strip |

Use a shared ease-out curve such as `cubic-bezier(0.23, 1, 0.32, 1)`. Motion distances, scale, opacity, durations, and easing are tokens. No `transition: all`, animated layout dimensions, per-item staggering, or animation-frame JavaScript loops.

Keep shell geometry and row selection immediate. Do not add view push/pop motion in this pass: the current instance and full-tree replacement boundaries should not be confused with navigation identity. This avoids replaying effects on AI chunks or re-keying a form for appearance alone.

The sweep is one pseudo-element inside a stable edge strip, animated with transform/opacity. Do not rotate a full-window conic gradient or animate a large blurred shadow. Only active busy surfaces render it. Existing reset/unmount paths stop it on hide or abandonment; also disable effects when the document is hidden. Do not assume a hidden warm webview will stop animations automatically.

Apply `prefers-reduced-motion` after token/theme rules. Reduced motion removes transforms, sweeps, shimmer, and pulses and shows a static edge/indicator. Panel visibility can change immediately; state information never depends on an animationend callback or nonzero duration.

### 7. Busy feedback consumes existing state truthfully

Root invocation feedback remains tied to `workingTitle`. List/detail feedback consumes the existing `loading` boolean outside the empty-content branch. Root result events have no aggregate loading/completion signal, so do not invent a root-search sweep or completion timer.

Keep the latest supplied items/text visible. A loading flag alone must not substitute skeletons for real content. Do not retain stale content against an explicit replacement tree; full-tree replacement still owns what content exists.

Mount a stable status presentation in a reserved portion of the existing footer/chrome so busy appearance does not resize the result viewport or shift form fields. At compact widths, wrap/truncate status and hints within their reserved areas while keeping the action trigger reachable. An empty loading list uses the existing "Loading…" empty-area message without a duplicate visible status message. A populated list or detail view uses "Loading…" in the status area. The busy edge and accessibility semantics are shared.

The AI producer's existing "Working…" content remains ordinary protocol content; do not special-case or rewrite it in the renderer. It can coexist with the common loading status. A later copy/producer change can remove that redundancy without coupling this renderer to a specific extension.

Set `aria-busy` on the affected content region and place a stable polite status region outside that busy subtree, so its announcement is not suppressed until work ends. Update status at state transitions, not per chunk. Do not make the entire streamed detail body a live region.

The installed `Command.Loading` defaults to numerical progress and exposes `aria-valuenow`; it is not a truthful fit for an unknown-duration boolean. Use native status semantics rather than claiming zero percent. The decorative sweep is a small CSS treatment, not a new behavior library. No new animation dependency is justified.

### 8. Copy and accessibility boundaries

Use the ux-writing guidance and existing interface-copy requirements. Preserve existing feature-provided labels, descriptions, errors, and action titles.

Strings reused in new locations or accessibility labels:

| String | Location |
| --- | --- |
| "Loading…" | Populated list/detail status, static reduced-motion status; existing empty-loading text remains |
| "Running {command title}…" | Root pending invocation status, unchanged wording |
| "Actions" | Visible footer trigger and accessible name for the action command scope |
| "Search apps and commands" | Existing root placeholder, also explicit accessible search label |
| "Search" | Existing pushed-list placeholder, also explicit accessible search label |

No completion toast, new error sentence, tooltip copy, or renamed action is introduced. Ellipses indicate ongoing work, not decorative punctuation. Dynamic content is interpolated as text, never HTML.

Associate shared field labels, help, and error IDs with controls; preserve password types, text selection behavior, multiline Enter, and platform-modified Enter submission. A Checkbox must be focusable if it is the first field; focus setup must not assume every first control supports `setSelectionRange`.

Keep enabled metadata readable in both built-in palettes. Expose disabled/invalid states in shared controls without inferring destructive intent from action IDs or title text: the protocol has no destructive-action styling field, and this change will not invent one.

## Risks / Trade-offs

- [Popover exit events reach the parent launcher] -> Test immediate and repeated Escape/Enter, pointer activation, focus return, and outside clicks while exit presence is active. Keep keyboard ownership explicit.
- [Command without a visible input has incomplete screen-reader feedback] -> Verify the focused command scope's selected-option relationship with NVDA and VoiceOver; keep selection/navigation owned by Bits while supplying only the missing accessibility relationship.
- [Centralization accidentally resets state] -> Preserve component lifetimes, stable result IDs, form keying, input refs, invocation abandonment, and the carried-failure behavior. Test delayed provider results and repeated detail replacements.
- [A theme looks coherent but becomes unreadable] -> Contrast checks against actual composited backgrounds, including selected/hover states and enabled metadata; do not merely compare opaque token values.
- [Shadows, portals, or focus rings clip at the webview boundary] -> Use inner depth, collision padding, and native-window verification. Do not expand the native window for an effect.
- [Animation consumes hidden-window resources or restarts per chunk] -> Mount once per busy lifecycle, use compositor-friendly effects, remove on reset/unmount, and inspect native performance traces.
- [New abstraction becomes a second framework] -> Extract only repeated presentation and real primitive compositions. Keep application state and protocol conversion at feature boundaries.
- [Browser fixtures miss native focus behavior] -> Use browser tests for repeatability, then verify real Windows and macOS builds. Browser success does not close native verification tasks.
- [Spec and implementation already differ on markdown and some lifecycle details] -> Do not silently resolve unrelated gaps in this polish change. Preserve existing content rendering; the loading delta intentionally addresses only missing busy feedback.

## Migration plan

1. Establish small development-only render fixtures and capture current navigation, focus, and appearance before changing presentation. Stub Tauri IPC for frontend fixtures rather than requiring real commands or credentials.
2. Add token layers with compatibility aliases matching the current appearance. Keep the app runnable and check/build after the step.
3. Extract shared command, field, frame, footer, keycap, and status presentation without changing event ownership. Verify identity, focus, icons, and form submission.
4. Migrate the action panel as one contained step, using the focused interaction fixture to prove keyboard, accessibility, portal, and exit behavior.
5. Apply the neutral visual values and state treatments, then the busy indicator and reduced-motion rules. Verify at 720x400 and a 480x300 development stress viewport; the latter is not a new native window-size setting.
6. Run type/build checks, Svelte component analysis, automated browser regression checks, and native Windows/macOS visual, accessibility, and timing verification. Preserve readable evidence and leave unavailable platform checks unticked.

Each group leaves a runnable app and can be reverted independently. No stored data, configuration, or protocol migration is needed. Roll back appearance through token values or revert the corresponding component step; do not revert favicon functionality incidentally.

## Reference evidence

- Bits UI transitions: https://bits-ui.com/docs/transitions
- Bits UI Command: https://bits-ui.com/docs/components/command
- Bits UI Popover: https://bits-ui.com/docs/components/popover
- Installed `bits-ui` Command types, root, loading component, and state implementation under `node_modules/bits-ui/dist/bits/command/` were inspected for pointer-selection controls, keyboard behavior, root focusability, and progress semantics.
- Svelte code-writer skill was read from the upstream path recorded by `skills-lock.json`: https://raw.githubusercontent.com/sveltejs/ai-tools/main/plugins/claude/svelte/skills/svelte-code-writer/SKILL.md. The local skill entries point to missing files; do not silently repair skill installation as part of UI work.

Exact colors, shadow strengths, and optical alignment remain visual tuning within the confirmed direction. No live screenshot or native rendering review has been performed during planning.
