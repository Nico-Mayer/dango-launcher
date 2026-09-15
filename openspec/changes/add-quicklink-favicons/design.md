## Context

See proposal.md - Why. What shapes the approach:

- `dango.quicklinks` and `dango.snippets` are one type, `SnippetsExtension`,
  parameterised by `Kind`. Only the quicklink half gets any of this.
- The icon field of a search candidate already carries either `icon:<name>` or a
  path to an image file, and `ResultRow.svelte` already renders both. The
  `applications` extension is the precedent: an `IconCache` writes files to the
  app cache directory, a background service fills it, and the provider reads a
  path per result.
- Root providers have a 50ms budget and run per keystroke, so the search path
  may not touch the network and should not touch the disk either.
- Records are file-backed and hand-editable, with a watcher that reloads them,
  so the set of hosts changes while the app runs.
- Dango makes no outbound request today other than to the AI provider the user
  configured. Adding one to the sites in the quicklinks file is a real change in
  what the app does, so it needs an off switch.

## Goals / Non-Goals

**Goals:**

- One favicon per host, fetched once, reused by every quicklink on that host.
- Nothing new on the search path: an in-memory lookup, no syscall, no request.
- The same icon in root search and in the Search Quicklinks list.
- Fetching that can be turned off, and that stops when the extension is disabled.

**Non-Goals:**

- Favicons for anything other than quicklinks. Snippets have no URL.
- A user-visible refresh or "reload icon" action. The refresh age covers it.
- Picking the best of several declared icons by resolution. Take a usable one.
- Any manifest or view protocol change. Neither contract moves here.
- A UI surface, so no Bits UI primitive is involved. The only frontend edit is
  one Tailwind class on an existing `<img>`.

## Decisions

### Where the icon comes from: the site itself, not a favicon service

Fetch from the host the quicklink points at. Rejected: Google's
`s2/favicons`, DuckDuckGo's `icons.duckduckgo.com`, and `icon.horse`, which
would be one request returning a normalised PNG and no HTML parsing at all. They
hand the list of sites the user keeps quicklinks for to a third party, which is
worse than the requests the user's own browser already makes to those same
sites. The convenience is not worth it for a launcher whose records live in the
user's dotfiles.

### Discovery and fetch: `favicon-picker`, with a narrow fallback

The project rule is off-the-shelf before hand-rolled. Candidates checked on
crates.io:

| Crate                  | Latest     | Verdict                                                         |
| ---------------------- | ---------- | --------------------------------------------------------------- |
| `favicon-picker` 1.1.0 | 2024-08    | Chosen. Parses `<link rel="icon">`, has a `/favicon.ico` default, has a blocking variant, and returns bytes. |
| `site_icons` 0.6.4     | 2023-01    | Dormant two years, and its selling point is WASM, which is not this. |
| `website-icon-extract` | small      | Extracts link paths only; we would still write the fetch.        |
| `webicon`              | unmaintained | Predates async reqwest entirely.                                |
| `favilib`              | small      | Writes to a path of its choosing; no say over the cache layout.  |

`favicon-picker` declares `reqwest ^0`, so Cargo resolves it onto the 0.13
already in the lock through `genai` rather than compiling a second copy. It adds
`scraper`, whose `html5ever` is already in the graph through `dom_query`.

Fallback if it does not compile against reqwest 0.13 (it was published against
0.12 and `^0` lets Cargo pick either): call `extract_icon_links_from_html`,
which is pure parsing with no HTTP in it, and make the two requests with our own
`reqwest::blocking` client. That keeps the discovery rules off the shelf and
costs only the request code.

Rejected: parsing the `<link>` tags ourselves with `dom_query`, which is already
in the graph. It is maybe thirty lines, but the rules for which `rel` values
count and how a relative href resolves are exactly what a library should own.

### What gets stored: the bytes as they arrived, not a decoded PNG

Write the response body with an extension taken from its content type: `.png`,
`.ico`, `.svg`, `.jpg`, `.webp`, `.gif`. Both WebView2 and WKWebView draw all
six in an `<img>`, so no decoding step is needed. Rejected: decoding to a 64px
PNG with the `image` crate (already in the graph through `clipboard-rs`, so
cheap to add) to match `IconCache`'s convention. It would need `ico`, `png`,
`jpeg` and `webp` features plus a resize, and it still could not handle the SVG
favicons a good number of sites now serve, which would need `resvg` on top. The
webview is already a competent image decoder; use it.

Anything that is not one of those six types is dropped, as is a body over
256KB. That is the ceiling on what a favicon can reasonably be, and it keeps a
misconfigured host from writing a large file into the cache.

### Cache: files in the app cache directory, freshness from mtime

`app_cache_dir()/favicons`, added to the asset protocol scope next to the
existing `icons` and `clipboard` directories. One file per host, named from the
host with the same sanitiser `IconCache` uses.

A host that answered with no usable icon gets an empty `<host>.miss` file, so a
failure is remembered rather than retried on every start. Freshness comes from
file mtime: an icon older than 14 days is refetched, a miss older than 24 hours
is retried. No new table and no new record type. The cache is disposable - if it
is deleted, everything refetches and nothing is lost.

Rejected: a SQLite table for cache metadata. Quicklinks were deliberately moved
out of the database and into a text file; putting their icons' bookkeeping back
in it would be a step backwards, and mtime already answers the only question
asked.

### The search path reads a map, not the disk

`FaviconCache` holds `RwLock<HashMap<String, PathBuf>>`, built with one
`read_dir` when it opens and updated by the service as it writes. The provider
resolves a record's host and does one map lookup. `IconCache::path` stats the
file per result, which is fine for the tens of applications a machine has but
would be 500 stats per keystroke at the quicklink count the existing spec puts a
budget on.

### The service: one thread, same shape as `IndexingService`

Started on activate, stopped on deactivate, a `std::thread` with an
`AtomicBool`, ticking every 200ms. Each sweep: read the records, resolve each to
a host, ensure a favicon for each host it has not seen or whose file is stale.
It re-sweeps when the set of hosts changes (the file watcher can reload records
at any time) and otherwise every 6 hours. Requests are sequential with a 5s
timeout each; there is no reason to fetch a hobbyist's quicklinks in parallel.

The preference is read at the top of each sweep rather than at start, so turning
it off takes effect within a tick without a restart, the way the clipboard
extension reads its exclusions fresh.

Blocking reqwest rather than the tokio runtime, because the service is a thread
and the existing background service is too. `reqwest::blocking` is a feature of
the crate already in the graph, not another dependency.

### Host resolution

A record's URL template is rendered with empty values - the same
`render_url(&Values::default())` the store already validates with - then parsed,
and the host taken from it. So a template with `{{query}}` in its path or query
string resolves to the host it would open. Only `http` and `https` are fetched.

### What the extension declares

`manifest(kind)` gains `services: kind == Kind::Quicklink` and, for that kind
only, one preference: key `favicons`, boolean, default `true`. `SnippetsExtension::new`
takes an `Option<Arc<FaviconCache>>` and an `Option<Preferences>`, both `None`
for snippets.

### The generic icon, and a slip fixed on the way

The fallback is `icon:link`, the quicklink icon, tinted pink as today; a file
icon is drawn untinted, which is already how application icons behave. That
contrast is part of what makes the row easier to pick out.

`list_tree` currently hardcodes `icon:clipboard-type` for both kinds, so the
Search Quicklinks list shows snippet icons. It moves to the same per-record icon
choice the provider uses.

### Frontend

One change: the non-content `<img>` branch in `ResultRow.svelte` gets
`object-contain`, so a favicon that is not square is letterboxed into the 32px
box rather than stretched. Application icons are square, so nothing there
changes. No new component, no primitive, no protocol change.

### User-facing text

No new on-screen string. Every failure here is silent by design - the spec says
a quicklink stays usable and the failure goes to the log, which is developer
text, not interface copy. The one piece of prose the user reads is the comment
in `docs/config.example.jsonc`:

```jsonc
"dango.quicklinks": {
  "preferences": {
    // Show each quicklink with its site's icon. Turning this off stops Dango
    // fetching anything from the sites your quicklinks point at. Default true.
    "favicons": false
  }
}
```

Sentence case, says what turning it off does rather than only what it is, no
identifiers on screen.

## Risks / Trade-offs

- [`favicon-picker` does not compile against reqwest 0.13] → Use only its
  `extract_icon_links_from_html`, which is pure parsing, and make the requests
  with our own blocking client. Decided above, so it is a small pivot, not a
  redesign.
- [The launcher now talks to the internet on its own] → Off by one preference,
  off entirely when the extension is disabled, and never on the search path. The
  hosts contacted are exactly the ones the user wrote into their own quicklinks
  file.
- [A webview refuses a format, leaving an empty box] → Only the six types every
  webview draws are stored; anything else is treated as a miss and the generic
  icon stays. Worth a look on both platforms during verification, since ICO in
  particular is the format the check rests on.
- [A favicon looks wrong at 32px, or is a 1-bit legacy `.ico`] → Accepted. It is
  still more distinguishable than an identical glyph on every row.
- [The cache grows without bound] → It cannot meaningfully: one small file per
  host the user keeps a quicklink for, and hosts are not pruned because that
  costs a `read_dir` walk to save kilobytes. If the file count ever matters,
  deleting the directory is a safe reset.
- [A site that redirects its favicon somewhere slow] → 5s timeout per request,
  sequential, on a background thread. The worst case is that an icon shows up
  late, which the spec already allows.
- [A host serving a tracking pixel as its favicon] → The request is made once
  per refresh period from a plain client with no cookie jar, so it is a weaker
  signal than the browser visit the quicklink exists to make.

## Migration Plan

Nothing to migrate. The cache starts empty, fills in the background, and every
quicklink works exactly as before while it does. Rollback is deleting the
favicons directory and the preference; no stored record changes shape.
