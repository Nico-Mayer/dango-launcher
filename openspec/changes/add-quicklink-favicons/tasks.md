## 1. Dependency and cache

- [x] 1.1 Add `favicon-picker` and `reqwest` (blocking, rustls) to
      `src-tauri/Cargo.toml`, then verify with `cargo tree -d | rg reqwest` that
      only one reqwest version is compiled and `cargo check` passes. If
      `favicon-picker` does not build against reqwest 0.13, fall back to the
      plan in design.md (its `extract_icon_links_from_html` plus our own client)
      and record which path was taken in this task.
      Done: `favicon-picker` builds against reqwest 0.13.5, so no fallback was
      needed. The now-redundant reqwest 0.12 dev-dependency was removed, leaving
      one reqwest in the tree.
- [x] 1.2 Add `src-tauri/src/extensions/snippets/favicons.rs` with
      `FaviconCache`: open a directory, index it with one `read_dir`, `path(host)`
      from the in-memory map, `write(host, bytes, content_type)`, `mark_miss(host)`,
      and staleness from mtime (14 days for an icon, 24 hours for a miss). Verify
      with unit tests over a temp directory: an unknown host has no path, a
      written icon is found by `path`, a fresh icon is not stale, a miss file
      keeps `path` empty and is retried once its age passes.
- [x] 1.3 Add content-type mapping and the 256KB body cap to the same module,
      and verify with unit tests that each of png, ico, svg, jpeg, webp and gif
      maps to its extension, that an HTML or unknown type is refused, and that an
      oversized body is refused.
- [x] 1.4 Add host resolution for a record (render the template with empty
      values, parse, take the host, accept only http and https) and verify with
      unit tests over a template with `{{query}}` in the path, one with it in the
      query string, a plain URL, and a non-http scheme.

## 2. Showing the icon

- [x] 2.1 Give `SnippetsExtension` an optional `FaviconCache` and have
      `SnippetProvider::items` use the cached path when there is one and
      `icon:link` otherwise. Verify with a test that a record whose host has a
      cached file gets that path and one without gets the named icon.
- [x] 2.2 Fix `list_tree` to use the same per-record icon choice instead of the
      hardcoded `icon:clipboard-type`, and verify with a test that a quicklink
      list item never carries the snippet icon.
- [x] 2.3 Add `object-contain` to the non-content `<img>` branch of
      `src/lib/ResultRow.svelte` and verify a non-square image is letterboxed
      rather than stretched, with application icons unchanged.

## 3. Fetching in the background

- [x] 3.1 Add the fetching service to `favicons.rs`: a thread with an
      `AtomicBool` following the `IndexingService` shape, sweeping the records'
      hosts, ensuring each icon that is missing or stale, sequential requests
      with a 5s timeout, re-sweeping on a changed host set and every 6 hours.
      Verify with tests over a fake fetcher that a sweep fetches an unknown host
      once, skips a fresh one, retries a stale one, and that `stop` ends the loop.
- [x] 3.2 Declare the service and the `favicons` boolean preference (default
      true) on the quicklink manifest only, and verify with a test that the
      snippet manifest declares neither and the quicklink manifest declares both.
- [x] 3.3 Read the preference at the top of each sweep so turning it off stops
      the requests within a tick, and verify with a test that a sweep with the
      preference off makes no call to the fetcher.
- [x] 3.4 Record a failed fetch to the log rather than to the screen, and verify
      with a test that an unreachable host leaves a miss marker, no panic, and
      the record still resolving to the named icon.

## 4. Wiring

- [x] 4.1 In `src-tauri/src/lib.rs`, create the `favicons` cache directory under
      `app_cache_dir()`, add it to the asset protocol scope, and pass the cache
      and a `Preferences` reader to the quicklink extension only. Verify by
      running the app, creating a quicklink, and seeing the file appear in that
      directory.
      Done: the running dev app wrote `github.com.svg` into
      `~/Library/Caches/com.nimayer.dango/favicons/` seconds after start.
- [x] 4.2 Add the `favicons` preference to `docs/config.example.jsonc` with the
      comment quoted in design.md, and verify the file still parses with
      `npx jsonc-parser` or by starting Dango against it with no config warning.
      Done: verified by `config::tests::the_example_config_validates_and_parses`,
      which parses the example and validates it against the schema.

## 5. Verification

- [ ] 5.1 Verify on macOS: quicklinks for at least four sites, at least one of
      them serving an `.ico` and one an `.svg`, all showing their favicons in
      root search and in Search Quicklinks; a brand new quicklink picking up its
      icon without a restart; the icons still there after a restart with no
      refetch.
- [x] 5.2 Verify on Windows: the same list, with attention to ICO rendering in
      WebView2 and to the cache directory resolving under the app cache path.
      Done: release build, seven quicklinks over github.com (svg),
      www.google.com (ico), en.wikipedia.org (ico), developer.mozilla.org (ico),
      news.ycombinator.com (svg), www.rust-lang.org (png) and a placeholder
      template on Wikipedia. WebView2 drew every ICO, and root search and Search
      Quicklinks showed the same icons, with no snippet glyph anywhere. The
      cache resolved to `%LOCALAPPDATA%\com.nimayer.dango\favicons\`, and the
      four new hosts were written within two seconds of start without the
      already cached github.com and www.google.com being touched.
- [ ] 5.3 Verify on both platforms that the launcher stays inside its budget
      with 500 quicklinks stored, using `env DANGO_MEASURE=1` and a generated
      quicklinks file, and that no request goes out while typing.
      Windows done: release build with `DANGO_MEASURE=1` and a generated file of
      500 quicklinks over eight hosts. 26 activations by the global hotkey: min
      17.7ms, median 23.9ms, p90 29.2ms, max 37.0ms, none over 80ms. Twenty
      queries typed against the 500 records left the process with zero TCP
      connections at every sample, the cache directory unchanged, and no new log
      line. macOS still to run.
- [x] 5.4 Verify on both platforms with the machine offline and with a quicklink
      pointing at a host that does not exist: no message on screen, every
      quicklink still opens, the failure in `dango.log`, and no repeated attempt
      on the next start.
      macOS done: a quicklink to `this-host-does-not-exist.invalid` left a miss
      marker and `no favicon for this-host-does-not-exist.invalid: error sending
      request ...` in the log, with nothing on screen. A live host that refuses
      every request (chatgpt.com, 403 on both the page and /favicon.ico) behaves
      the same.
      Windows done: the same quicklink left the miss marker and the log line,
      nothing on screen, and Enter on it still opened the browser. A restart ten
      seconds later left the cache directory and the log byte for byte unchanged.
- [x] 5.5 Verify on both platforms that setting `"favicons": false` stops the
      fetching within a tick and returns every quicklink to the generic icon,
      and that turning it back on restores them without a restart.
      macOS done, and it found a bug: the sweep woke only on a records edit or
      the 6-hour timer, so turning the preference back on did nothing until one
      of those. The wait now watches the preference too, and a sweep already
      running stops as soon as it goes off. Re-verified live: a quicklink added
      while off was not fetched, and was fetched about two seconds after turning
      it on.
      Windows done: turning the preference off while running returned every
      quicklink in Search Quicklinks to the link glyph, and a quicklink added
      while off was not fetched in five seconds. Turning it back on fetched that
      host half a second later and the icons came back without a restart.
- [x] 5.6 Run `cargo clippy -- -D warnings`, `cargo test`, and `npm run check`,
      and verify all three pass.
