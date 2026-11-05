# Tasks

## 1. Web search flag on quicklinks

- [x] 1.1 Add `fallback` to `QuicklinkEntry` and `Record`, read it as a bool in `project()`, write it through `Records::save` with `toml_edit` (removed when false), and add a file-order accessor to `Records`. Verify: store tests in `src/extensions/snippets/store.rs` cover reading `fallback = true`, an entry without the key, `fallback = "yes"` reported with its line, saving with and without it (key removed on unmark), comments and order surviving the write, and the accessor returning file order; the `quicklinks.toml` schema test sees the new kebab-case key
- [x] 1.2 Add the helper deciding a usable web search (marked and `prompts()` exactly `["query"]`), the "Use for web search" toggle on the quicklink form (not the snippet form), and the save refusal "For web search, the URL must ask only for {{ query }}". Verify: tests in `src/extensions/snippets/extension.rs` cover the toggle in the create and edit forms, saving it on and off, refusal for a URL without `query` and for one with another argument (nothing stored), a URL with `query` and `clipboard` accepted, and the snippet form having no toggle

## 2. Web search rows in root search

- [x] 2.1 Add `Source::WebSearch` and split it off in `DangoRanker::rank`: rank the rest with the limit minus answers and web searches, then return answers, ranked results, web searches. Verify: tests in `src/ranking/mod.rs` cover web searches staying last behind an exact match and a frecent item, other results being cut before them at the limit, all web searches kept with a calculator answer at the limit, and their order kept
- [x] 2.2 Drop `WebSearch` candidates from every incomplete emit in `run_query`. Verify: tests in `src/search/mod.rs` cover a slow provider: early emits hold no web search, the complete emit holds them last
- [ ] 2.3 Emit one web search candidate per usable web search record from the quicklinks root provider, in file order, for a non-empty query only, skipping ids that contain `:`, with id `web:<record-id>:<query>`, the record's icon, title `Search <name> for “<query>”`, and actions "Open" and "Copy". Verify: provider tests in `src/extensions/snippets/extension.rs` cover the title, the favicon, file order (not name order), no rows for the empty query, no row for an unusable marked entry or an id with `:`, and the quicklink itself still emitted as a normal row
- [ ] 2.4 Handle `web:` ids in `perform_action`: set the `query` argument from the id and go straight to open or copy, without a form. Verify: tests cover opening with the query encoded (`a&b?c`), opening in the named browser, a template with `clipboard` filled without a form, and copying the rendered URL
- [ ] 2.5 Add `web_search` to `ResultItem`, set in `Core::result_item`, and put these rows in a "Search the web" section in `root_screen`. Verify: state tests in `src/ui/tests.rs` cover the heading under the ranked rows with no other heading on a typed query, the first web search selected with no "No results" when nothing else matches, and "No results" still shown with no web search
- [ ] 2.6 Skip `record_use` for web search ids through a `pub` helper in the quicklinks module. Verify: a test in `src/app/tests.rs` confirms a web search row and checks frecency and learned inputs are unchanged
- [ ] 2.7 UI test on the Slint testing backend for a query with one ranked result and one web search: both rows render, the "Search the web" heading sits between them, and the ActionBar reads "Open". Verify: the new test in `src/slint_shell/tests.rs` passes

## 3. DuckDuckGo default

- [ ] 3.1 In `register_snippets`, when the quicklinks extension is enabled and `quicklinks.toml` is missing, create it through the records' write path with the `#:schema` header and the `[duckduckgo]` web search entry. Verify: tests cover a missing file being created with the entry, an existing file without the entry left untouched, and no file created when quicklinks are disabled; the created file loads back as one web search

## 4. Wrap-up

- [ ] 4.1 Add any platform trap or build fact found to AGENTS.md. Verify: `mise run check` and `cargo test` pass
- [ ] 4.2 Run the `ponytail-review` skill in a fresh subagent on this change's commits, without the apply context; report findings to the author

## Workflow follow-up

- Archive after review. The change is platform-neutral, so it has no hand checks to wait for.
