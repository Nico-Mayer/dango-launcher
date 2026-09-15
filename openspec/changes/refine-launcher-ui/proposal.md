## Why

Dango's launcher works, but repeated styling, weak surface hierarchy, and incomplete loading feedback make it feel flat and static. A focused M7 polish change can make daily interactions clearer and more tactile while keeping the existing layout and keyboard speed.

## What changes

- Establish a theme-ready design system covering colors, spacing, typography, radii, elevation, control dimensions, icons, and motion. Keep built-in defaults; do not add theme file loading or a picker.
- Refine the existing appearance toward neutral graphite surfaces, restrained blue selection and focus, clear text hierarchy, and subtle depth. Preserve extension tints and image identities.
- Preserve the current density: 56px result rows and 64px search headers at the default text scale.
- Introduce dedicated shared UI components and compose feature views from them, retaining Bits UI behavior and avoiding repeated styling.
- Replace the custom action panel's navigation and positioning with Bits UI composition while preserving the existing selection and keyboard contracts.
- Keep launcher activation and keyboard selection immediate. Add restrained overlay and hover transitions, plus one reusable loading sweep with a static reduced-motion alternative.
- Show loading for both empty and populated lists and detail views without clearing existing content, replaying entrance effects on streamed updates, or inventing progress percentages.

## Capabilities

### New capabilities

- `visual-system`: Cohesive, theme-ready presentation, preserved density, accessible states, restrained motion, and consistent behavior across shared surfaces.

### Modified capabilities

- `view-protocol`: Clarify loading feedback for populated lists and detail views, including streaming updates and accessible busy state. No protocol shape or version change.

## Impact

- Frontend: `src/app.css`, `src/App.svelte`, and existing components under `src/lib/`, with shared presentation under `src/lib/ui/`, launcher composition under `src/lib/launcher/`, and token definitions under `src/lib/theme/`.
- Dependencies: reuse Svelte 5, Tailwind v4, Bits UI, the existing animation utilities where useful, and the existing icon provider. No new production dependency planned.
- Contracts: extension manifests, view protocol version 1, backend ranking, invocation ownership, configuration files, native window geometry, and persistence remain unchanged.
- Platforms: Windows and macOS, including native webview rendering, focus behavior, reduced motion, and activation latency verification on each.
- Milestone: M7 - polish in `openspec/ROADMAP.md`; this change does not attempt the rest of M7.
- Coordination: preserve the active `add-quicklink-favicons` change's image containment and fallback behavior when refactoring `ResultRow.svelte`.

## Non-goals

- A full rewrite, new routing, new backend business logic, or protocol extensions.
- Loading, validating, watching, or selecting a user-provided `theme.json`; a public theme schema or theme editor.
- Preferences windows, permission onboarding, autostart, stats dashboards, or speculative tabs and dialogs.
- Markdown rendering, AI accounting, new commands, or new data models.
- Native window animation, blur/vibrancy, exterior shadow geometry changes, or different default density.
- Staggered result entrances, animated keyboard selection travel, bouncing controls, or decorative effects while idle.
