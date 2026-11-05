# Tasks

## 1. Web search flag on quicklinks

- [ ] 1.1 Add `fallback: bool` to `QuicklinkEntry` (absent = false, not written when false) and carry it through `Records::save` and `Record`. Verify: store tests in `src/extensions/snippets/store.rs` cover reading `fallback = true`, an entry without the key, saving with and without it (the key is removed on unmark), and comments and order surviving the write; the `quicklinks.toml` schema test sees the new kebab-case key
- [ ] 1.2 Add the "Use for web search" toggle to the quicklink form (not the snippet form) and refuse saving a web search whose URL does not use `query` with "Add {{ query }} to the URL to use it for web search.". Verify: tests in `src/extensions/snippets/extension.rs` cover the toggle in the create and edit forms, saving it on and off, the refusal storing nothing, and the snippet form having no toggle

## 2. Web search rows in root search

- [ ] 2.1 Add `Source::WebSearch` and split it off in `DangoRanker::rank`: rank the rest with the limit minus the web searches, then append them in the order given. Verify: tests in `src/ranking/mod.rs` cover web searches staying last behind an exact match and a frecent item, other results being cut before them at the limit, and their order kept
- [ ] 2.2 Emit one web search candidate per marked quicklink whose URL uses `query` from the quicklinks root provider, for a non-empty query only, with id `web:<record-id>:<query>`, the record's icon, title `Search <name> for “<query>”`, and actions "Open" and "Copy". Verify: provider tests in `src/extensions/snippets/extension.rs` cover the title, the favicon, file order, no rows for the empty query, no row for a marked entry without `query`, and the quicklink itself still emitted as a normal row
- [ ] 2.3 Handle `web:` ids in `perform_action`: fill the `query` argument from the id, then run the existing open and copy paths. Verify: tests cover opening with the query encoded (`a&b?c`), opening in the named browser, copying the rendered URL, and a template with another argument asking only for that one
- [ ] 2.4 Add `web_search` to `ResultItem`, set in `Core::result_item`; place these rows last in `root_order` and in a "Search the web" section in `root_screen`. Verify: state tests in `src/ui/tests.rs` cover the heading and placement under ranked rows, late results landing above the heading, the first web search selected with no "No results" when nothing else matches, an unpicked web search giving up the selection to a late result, and a picked one keeping it; "No results" still shows with no web search
- [ ] 2.5 Skip `record_use` for web search ids through a `pub` helper in the quicklinks module. Verify: a test in `src/app/tests.rs` confirms a web search row and checks frecency and learned inputs are unchanged
- [ ] 2.6 UI test on the Slint testing backend for a query with one ranked result and one web search: both rows render, the "Search the web" heading sits between them, and the ActionBar reads "Open". Verify: the new test in `src/slint_shell/tests.rs` passes

## 3. DuckDuckGo default

- [ ] 3.1 In `register_snippets`, when the quicklinks extension is enabled and `quicklinks.toml` is missing, create it through the records' write path with the `#:schema` header and the `[duckduckgo]` web search entry. Verify: tests cover a missing file being created with the entry, an existing file without the entry left untouched, and no file created when quicklinks are disabled; the created file loads back as one web search

## 4. Wrap-up

- [ ] 4.1 Add any platform trap or build fact found to AGENTS.md. Verify: `mise run check` and `cargo test` pass
- [ ] 4.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive after review. The change is platform-neutral, so it has no hand checks to wait for.
