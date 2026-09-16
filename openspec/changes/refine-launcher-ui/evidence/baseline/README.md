# Stage 1 baseline evidence

## Environment

- macOS 26.6.2 (25G83), arm64; Node 24.21.0, npm 11.19.0.
- Playwright Test 1.63.0; Chromium 153.0.8010.12, headless shell revision 1243.
- Browser viewport: 720×400 CSS pixels, device scale factor 1, built-in dark palette.
- Browser-only evidence. No Tauri/native-window, Windows, VoiceOver, NVDA, display-scaling,
  or activation-latency claim is made.

## Commands and outcomes

| Command | Outcome |
| --- | --- |
| `npm install -D @playwright/test` | Installed development-only runner; audit: 0 vulnerabilities. Optional fsevents install-script warning did not affect checks. |
| `npx playwright install chromium` | Passed after initial browser launch reported missing revision 1243; no system dependencies installed. |
| `npm run test:browser` | 28/28 passed, 5.0s. |
| `npm run test:browser -- --repeat-each=3` | 84/84 passed, 11.4s. |
| `npm run check` | 0 errors, 0 warnings, including fixture/test TypeScript. |
| `npm run build` | Passed; 727 modules, JS 112.29 kB (gzip 37.23 kB), CSS 19.65 kB (gzip 4.56 kB). |
| `find dist -type f` | Only index.html plus production JS/CSS. No fixture HTML/module emitted. |
| `grep -R -E 'dangoFixture\|Browser fixtures\|fixture-only\|Unstubbed fixture command' dist` | No matches (exit 1, expected); fixture data/API absent from production bundle. |
| `npx @sveltejs/mcp svelte-autofixer <file>` | No issues for App, ProtocolView, FormFields, ActionPanel, ResultRow, Icon, input.svelte.ts. Existing effect advisories in App/ProtocolView/FormFields retained: scroll/focus/IPC synchronization is intentional, production components unchanged. |

Official installed Tauri `mocks.d.ts`/`mocks.js` checked for `mockIPC` and
`shouldMockEvents`; Playwright configuration/screenshot APIs checked against
Context7 `/microsoft/playwright`. No custom IPC transport or navigation implementation.

## Coverage

- Ten fixtures load real App with stubbed invocation/events, no page errors and no external
  requests. Detail fixture accepts timed full-tree streaming updates.
- Root and pushed lists: both arrow boundaries do not wrap; last row scrolls into view;
  returning to first row restores scrollTop=0 and root Suggestions heading.
- Pointer hover never changes selected identity. Keyboard removes hover paint immediately;
  equal-coordinate pointer events do not revive it; real movement restores it.
- Root and pushed rows retain search focus after click and subsequent typing.
- Delayed/reordered root results and pushed trees preserve selected ID; removing that ID
  selects first remaining row. Stale root-query results are rejected.
- Text-field Enter submits once with edited and untouched initial values.
- Template Enter inserts newline; both Ctrl+Enter and Cmd+Enter submit once.
- Failure before/after reset survives activation; typing, acting, and Escape each clear it.

## Screenshots

Each PNG is 720×400: `root`, `list`, `empty-list`, `loading-list`, `refreshing-list`,
`detail`, `form`, `template`, `failure`, `long-actions`.

Root, form, and long-actions screenshots were opened for visual inspection. Original
source styling is retained. Known design inputs remain visible: long custom action panel
extends above viewport; populated loading list/detail do not yet show common loading
status; metadata remains faint. These are planned later-stage changes, not assertions
that stage 1 fixed them. No unexpected baseline navigation/focus failure was found.

Screenshots are historical evidence, not pixel-golden assertions. Re-running fixture tests
writes current screenshots under ignored `test-results/`, leaving this baseline intact.
