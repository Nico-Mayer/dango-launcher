## 1. Configuration and keys

- [x] 1.1 Add the `providers` branch to the config `Extension` struct (name, `kind` of `anthropic`/`openai`/`ollama`, optional `baseUrl`, optional `model`, unknown keys preserved) and verify a round trip through `Config::parse` and `to_json` keeps an unrelated key
- [x] 1.2 Add the AI fields to `CommandSettings` (`title`, `prompt`, `provider`, `model`, `thinking`, `output`, `enabled`) as optional, and verify `hotkey` and `alias` still parse and write back unchanged
- [x] 1.3 Add the `auth.json` reader: resolve it from the config directory honouring `$DANGO_CONFIG_DIR`, parse a map of provider name to key, treat a missing file as no keys, and verify with tests for present, missing, and malformed files
- [x] 1.4 Restrict `auth.json` to its owner when reading it on macOS, leaving it alone on Windows, and verify with a unix-only test that a world-readable file is tightened
- [x] 1.5 Verify by inspection and a test that no key reaches the log, `config.json`, or any message: the reader's error type carries the file, never the contents

## 2. The provider seam

- [x] 2.1 Add the `genai` dependency pinned to an exact minor version and verify `cargo build` succeeds on macOS and in CI on Windows
- [x] 2.2 Define the `Completions` trait, `Request` (endpoint, model, thinking, prompt, key), `Chunks`, and `AiError`, with a scripted test implementation, and verify a test drives a fake stream end to end without the network
- [x] 2.3 Implement the `genai` adapter: map a provider entry to its service target and model, supply the key per request, map the thinking level to the client's reasoning effort, and verify unit tests over the mapping for each of the three provider kinds
- [x] 2.4 Map failures to the messages quoted in the design (no key, rejected key, unknown model, unreachable, rate limited, timed out), keeping the raw error for the log only, and verify a test per case
- [x] 2.5 Abandon a request that has produced nothing within 30 seconds and verify a test reports the timeout message

## 3. The AI extension

- [x] 3.1 Add `src-tauri/src/extensions/ai/` with the shipped command set as constants (id, Title Case title, prompt, default output) for the nine commands named in the proposal, and verify a test asserts every shipped prompt parses as a template
- [x] 3.2 Build the manifest from the shipped set merged with the config (override by id, `enabled: false` drops one, a title and prompt adds one, an entry that is neither is skipped and reported), and verify tests over each merge case
- [x] 3.3 Resolve a command's provider, model, and thinking level from the command, then the extension defaults, then the provider's default, and verify tests for each fallback step
- [x] 3.4 Render the prompt through `crate::templates`, reading the selection or the clipboard only when the prompt refers to them, and verify tests that an unreferenced selection is never read
- [x] 3.5 Ask for a prompt's arguments through a form view before any request, following the snippets extension's prefixed-field pattern, and verify a test that submitting the form renders the prompt with the values
- [x] 3.6 Report "Select some text first, then run this command." when the prompt needs the selection and there is none, with a test that no request is made
- [x] 3.7 Stream the answer: drive the client on the captured tokio handle from the invocation thread, buffer fragments, push a detail tree at most every 50ms plus a final one, and verify a test that thirty fragments produce far fewer trees and the last one holds the whole answer
- [x] 3.8 Show "Working…" with the view marked loading until the first text arrives, and verify a test on the first pushed tree
- [x] 3.9 Apply the `output` setting on completion (`view` leaves it on screen, `paste` hides and inserts, `copy` copies and hides) and offer "Paste" and "Copy" as the result view's actions, with tests over each outcome
- [x] 3.10 Report "The model returned no text. Try again." for an empty answer, with a test that nothing is pasted or copied

## 4. Live rebuild, cancellation, and wiring

- [x] 4.1 Add `ExtensionHost::replace` (deactivate, drop, insert, activate when enabled) and verify tests that the old commands are unregistered and the new ones registered, and that a disabled extension stays inactive
- [x] 4.2 Add `is_cancelled` to `InvocationContext` backed by a flag the `Invoker` trips on supersede and on cancel, and verify a test that a superseded streaming command stops pushing
- [x] 4.3 Add the `cancel_invocation` Tauri command, call it from the frontend when the launcher hides and when a streaming view is popped with Escape, and verify by running a long answer and pressing Escape that nothing further renders
- [x] 4.4 Build the AI extension in `lib.rs` from the config, the text target, and the provider client, register it, and verify the shipped commands appear in root search
- [x] 4.5 Rebuild and replace the extension in `apply_config_reload`, before the search candidates and hotkey table are rebuilt, and verify that a command added to the file appears and one removed disappears without a restart
- [x] 4.6 Verify `cargo test`, `cargo clippy -- -D warnings`, and `npm run check` are clean

## 5. Documentation and recorded context

- [x] 5.1 Document the `dango.ai` branch in `docs/config.example.jsonc` (providers, defaults, overriding a shipped command, disabling one, adding one) and verify the example stays valid JSON with its comments stripped
- [x] 5.2 Extend `docs/config.schema.json` for the new branch and command fields, and verify the schema test accepts the example and rejects a bad `kind`, `thinking`, or `output`
- [x] 5.3 Add `docs/auth.example.json` with placeholder keys and a line in the README saying to keep `auth.json` out of git
- [x] 5.4 Amend the keychain constraint in `openspec/config.yaml` to record that keys live in `auth.json`, with the keychain named as the later option, and verify the file still parses as YAML
- [x] 5.5 Mark M6 in `openspec/ROADMAP.md` with what shipped and what it deliberately left out

## 6. Verification on both platforms

- [ ] 6.1 Verify on Windows: a hosted provider with a key in `auth.json` and a local Ollama provider both answer, and the answer streams into the launcher
  - Ollama: done. Text selected in Notepad, Improve Writing run from root search,
    the launcher showed "Working…" and then the streamed answer with Paste and
    Copy in the footer. `cargo test a_local_ollama_answers -- --ignored` covers
    the same path without the launcher.
  - Hosted: not done, and cannot be until there is a key with credit on it. The
    three failure paths a key exercises are covered by 6.6.
- [ ] 6.2 Verify on macOS: the same two providers answer and stream, and the Accessibility permission path behaves as it does for snippets
- [ ] 6.3 Verify on Windows: a command set to paste replaces the selected text in another application, and one set to copy leaves the answer on the clipboard
  - Done, all three ways. The result view's Paste action replaced the selection
    and left the clipboard as it was. `output: "paste"` on Fix Spelling replaced
    it on its own with no view left behind. `output: "copy"` on a command going
    through the Claude CLI put the answer on the clipboard and hid the launcher.
- [ ] 6.4 Verify on macOS: the same paste and copy outcomes, with the selection replaced in the application the user came from
- [ ] 6.5 Verify on both platforms: a command added to `config.json` by hand appears without a restart, runs from its hotkey, and disappears when removed
  - Windows: done. Four commands added to the file while Dango ran appeared in
    root search with no restart, three of them ran from the chords that same
    edit gave them, and removing them from the file took them out of search
    live, again with no restart.
  - macOS: not done.
- [ ] 6.6 Verify on both platforms: a missing key, a wrong model name, and a stopped local server each show their message and leave the launcher usable
  - Windows: all three shown and correct, launcher usable behind each. This
    found a real bug: a failure arriving once the answer is already streaming is
    wrapped in `WebStream`, so the status was never read and an unknown model
    reported "couldn't be reached". Fixed, with tests, and re-verified.
  - macOS: not done.
- [ ] 6.7 Verify on both platforms: pressing Escape during a long answer stops the request and the launcher is responsive throughout
- [x] 6.8 Verify on Windows: a command provider answers a transform, and abandoning it leaves no process behind
  - Answering: done. A command routed to `claude -p` answered through the
    launcher and its `output: "copy"` left the message on the clipboard.
  - Abandoning: this found a real bug. The child was killed but its own children
    were not, so a wrapper script's actual work carried on. Fixed with
    `process-wrap` (job object on Windows, process group on Unix), covered by a
    regression test, and re-verified live: the grandchild count went 0, 1, 0
    across start and Escape.
- [ ] 6.9 Verify on macOS: the same, with the command the machine has installed

## 7. A provider that is a command

- [x] 7.1 Add the `cli` provider kind to the config with its `command` and `args`, and verify a round trip keeps them and that an entry with no command is reported rather than used
- [x] 7.2 Implement `CliClient` behind `Completions`: substitute `{prompt}` and `{model}` in the arguments, pass the prompt on standard input when no placeholder asks for it, run in an empty working directory, and stream standard output as it arrives, with tests over the substitution, the stdin default, and streaming
- [x] 7.3 Map a command that cannot be run, one that exits without an answer, and one still running when the answer is abandoned, to their messages and to a killed child, with a test per case
- [x] 7.4 Choose the client per request by provider kind, leaving the extension and every command unchanged, with a test that each kind reaches its own client
- [x] 7.5 Document the kind in `docs/config.example.jsonc` and `docs/config.schema.json`, and verify the example still validates
  - Windows: Escape mid-stream popped back to root search, no further tree from
    that answer appeared, and typing straight afterwards searched normally.
  - macOS: not done.
