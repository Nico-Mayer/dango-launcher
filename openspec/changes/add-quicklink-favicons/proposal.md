## Why

Every quicklink in root search wears the same pink `icon:link` glyph, so a list
of them reads as a column of identical rows and the user has to read each title
to find the one they want. The site's own favicon is the thing that tells them
apart at a glance, the way an application's own icon already does for the
`applications` extension.

Milestone: a follow-up to M3, which shipped `quicklinks`. It is polish in the
spirit of M7 and opens no new milestone.

Platforms: macOS and Windows, one code path. Nothing here is platform-specific:
a network fetch, a file cache, and an `<img>` in the webview, all of which the
launcher already does for application and clipboard-image icons. Linux stays out
of scope.

## What Changes

- A quicklink whose URL points at a site whose favicon Dango has fetched shows
  that favicon in root search and in the Search Quicklinks list, instead of the
  generic link glyph.
- Favicons are fetched by a background service on the `dango.quicklinks`
  extension, never on the search path. A quicklink with no favicon yet, or one
  whose site has none, keeps the generic glyph.
- The cache is keyed by host, so several quicklinks on the same site share one
  fetch and one file. It lives in the app cache directory next to the
  application icon cache, and survives restarts.
- A refetch happens when a cached favicon is older than a set age, and a host
  that failed is not retried for a while, so a site that is down or has no icon
  does not get hit on every start.
- A new `favicons` preference on `dango.quicklinks`, default on, turns the
  fetching off for a user who does not want the launcher making requests to the
  sites in their quicklinks file.
- Fixes an existing slip while in the same code: the Search Quicklinks list
  renders every row with the snippet glyph (`icon:clipboard-type`) rather than
  the quicklink one, because `list_tree` hardcodes it.

## Capabilities

### New Capabilities

None. This changes how an existing capability presents itself.

### Modified Capabilities

- `quicklinks`: a new requirement that a quicklink is shown with its site's
  favicon where one is available, with the generic icon as the fallback; a new
  requirement covering how favicons are fetched, cached, and refreshed off the
  search path, and the preference that turns it off.

## Impact

- `src-tauri/src/extensions/snippets/`: a new favicon module (fetch, cache,
  service), the root provider and `list_tree` choosing an icon per record, and
  the manifest declaring `services` and the new preference for the quicklink
  kind.
- `src-tauri/src/lib.rs`: a cache directory for favicons, added to the asset
  protocol scope the way the application icon cache already is, and passed to
  the quicklink extension at registration.
- `src-tauri/Cargo.toml`: an HTTP client and a favicon discovery crate. Both
  reqwest and the HTML parsing stack are already in `Cargo.lock` through `genai`
  and `tauri`, so the build cost is small.
- `docs/config.example.jsonc`: the new preference. `docs/config.schema.json`
  needs no change, since it takes preference values as free-form.
- No protocol change. The icon field already carries either `icon:<name>` or a
  file path, and `ResultRow.svelte` already renders both.
- New behaviour worth stating plainly: with the preference on, Dango makes
  outbound HTTP requests to the hosts in the user's quicklinks file. It made
  none before, apart from the AI provider the user configured.
