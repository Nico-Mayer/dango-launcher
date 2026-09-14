## Context

See `proposal.md` for motivation. What already exists and this change builds on:

- `crate::templates` parses `{{ name }}` templates, reports the arguments a
  template needs, and resolves `selection` and `clipboard` on demand.
- `crate::text::TextTarget` reads the selection and inserts text with focus
  restore and clipboard restore, on both platforms.
- `crate::invocation` starts a command on the `BuiltinHost` (a std thread),
  carries its output on a channel, and drops output from a superseded
  invocation. `InvocationContext::push_view` replaces the tree on screen.
- `crate::config` loads `config.json`, keeps unknown keys, watches the file, and
  re-applies it live through `apply_config_reload` in `lib.rs`.
- `crate::hotkeys::bindings` builds global hotkeys from the config's
  `extensions.<id>.commands.<id>.hotkey` entries, for whatever command ids it
  finds there.
- The protocol has a `Detail` view with `markdown` and `loading`, and a `Form`
  view the snippets extension already uses to ask for a template's arguments.

Two constraints shape the work: every request must be made in Rust, never in the
webview, and the launcher must stay responsive while a request streams.

## Goals / Non-Goals

**Goals:**

- One extension, `dango.ai`, whose commands are data the user can read and edit.
- One provider seam, so a second implementation (or a keychain) can replace a
  piece without touching a command.
- Streaming that reuses the existing full-tree-replace path with no protocol
  change.
- The shipped commands work after adding one provider and one key.

**Non-Goals:**

- No new view kind, no protocol version bump, no new frontend primitive.
- No change to how the selection is read or how text is pasted. That is M3's
  plumbing, used as is.
- No abstraction for "AI" beyond what these commands need. One request, one
  answer, text in and text out.

## Decisions

### Use the `genai` crate rather than writing two adapters

The roadmap said "a small provider trait with a native Anthropic adapter plus an
OpenAI-compatible adapter". The project's off-the-shelf rule says to look first,
and `genai` (0.6.5 stable, actively developed, MIT/Apache) is exactly that
library: native Anthropic protocol, an OpenAI-compatible adapter, a dedicated
OpenRouter namespace, Ollama as a first-class local provider, `exec_chat_stream`
for streaming, `ServiceTargetResolver` for a custom endpoint and model, and
`AuthResolver` for supplying a key at call time instead of from the environment.

Alternatives considered:

- **Hand-rolled adapters over `reqwest` plus an SSE parser.** Two request
  shapes, two streaming event formats, two error vocabularies, and a standing
  maintenance cost as both protocols move. Rejected: this is the case the rule
  exists for.
- **`rig-core`.** Broader provider coverage and more users, but it is an agent
  framework: agents, embeddings, vector stores, RAG, tool loops. Dango needs one
  streaming completion. Rejected as far more surface than the job needs.
- **`async-openai` plus an Anthropic client.** Two dependencies, two APIs, two
  streaming types to unify by hand, which is most of what `genai` already does.
  Rejected.
- **`llm-kit` / `llm-relay`.** Young and thinly used compared with the above.
  Rejected on maturity.

`genai` is edition 2024, so the CI toolchain must be recent enough; it is pinned
to an exact minor version because the 0.7 beta line is moving.

A thin internal trait keeps it swappable and keeps the tests off the network:

```rust
pub trait Completions: Send + Sync {
    fn stream(&self, request: Request) -> Chunks;
}
```

`Request` carries the resolved endpoint, model, thinking level, prompt, and key;
`Chunks` hands back one fragment at a time, ending in completion or an `AiError`.
A failure arrives through the same channel as the text rather than as a separate
return, because a request can fail after it has already said something.
The extension depends on the trait; the `genai` client is one implementation and
a scripted one is used in tests.

### Keys in `auth.json`, plain, with the keychain deferred

`openspec/config.yaml` records "secrets go in the OS keychain" as a
non-negotiable constraint. This change overrides it on the author's decision: one
user, two machines, and a file that can be copied beats a keychain round trip
right now. The constraint in `config.yaml` is amended in the same change so the
recorded context does not lie to a future session.

What keeps the blast radius small:

- The keys live in their own file, `auth.json`, never in `config.json`, so the
  configuration stays shareable and committable while the key file is not.
- Dango only reads the file. It never writes a key, never logs one, and never
  puts one in a message. Read errors name the file, not its contents.
- On macOS the file is restricted to its owner when read. Windows has no cheap
  equivalent through `std::fs`, so it is left alone there, which the spec says.
- The file is read per request rather than cached, so a key added while Dango
  runs works immediately and no key sits in memory between requests.
- The read is one function behind the provider seam, so swapping in the `keyring`
  crate (4.2, both platforms) later is a change to that function and nothing
  else.

### A provider may be a command

A subscription to ChatGPT or Claude buys inference that no API key can spend. The
vendor's own CLI is signed in to that subscription and will answer a prompt
non-interactively (`codex exec`, `claude -p`, `pi --print`), so the cheapest way
to reach it is to run the program the user already has.

That generalises past those three: a provider of kind `cli` names a command and
its arguments, and anything that takes a prompt and prints an answer qualifies.

```jsonc
"pi":     { "kind": "cli", "command": "pi",     "args": ["--print"] },
"claude": { "kind": "cli", "command": "claude", "args": ["-p"] },
"codex":  { "kind": "cli", "command": "codex",  "args": ["exec", "-"] }
```

Decisions inside it:

- **The prompt goes on standard input** unless `args` contains `{prompt}`, which
  is replaced instead. `{model}` is replaced the same way, so each CLI's own flag
  spelling stays in configuration rather than in Rust.
- **Raw standard output is the answer**, streamed as it arrives. Standard error is
  the program's chatter and goes to the log. Both `claude -p` and `codex exec`
  print the answer to stdout and progress to stderr, so this needs no per-program
  knowledge. A JSON Lines mode (`codex exec --json`) would need a path expression
  per program and is deliberately left out.
- **Dropping the stream kills the child, and everything it started.** Escape has
  to stop an agent, not orphan it. Killing only the process we spawned is not
  enough: the programs worth driving here are wrapper scripts, so the one doing
  the work is a grandchild, and it survives. `process-wrap` (10.0, the
  maintained successor to `command-group` by the same author) spawns into a
  Windows job object or a Unix process group and takes the tree down with one
  kill. Hand-rolling job objects and `killpg` is exactly the general job the
  off-the-shelf rule exists for.
- **The child runs in an empty temporary directory.** These programs are agents
  with file tools; a one-shot text transform has no business reaching the user's
  files. The constraining flags each one offers (`--sandbox read-only` and
  friends) stay in the user's `args`, because only they know the spelling.
- **No key.** Like Ollama, a command provider is never looked up in `auth.json`.

It is a second implementation of `Completions`, chosen per request by kind, so
commands, outputs, thinking levels, and the extension itself are untouched.

The cost is latency: process start plus agent warm-up is seconds against roughly
100ms for an HTTP call. That makes it a fit for Summarize or Explain This and a
poor one for a hotkey-driven Fix Spelling, so Ollama stays the fast path. The
`thinking` setting has no meaning here and is ignored.

**Rejected: carrying the vendor's first-party client identity.** A harness can
reach a ChatGPT subscription with no Codex installed by performing Codex's own
OAuth flow with its client id and calling the ChatGPT backend with the resulting
token. It works, and it is what tools advertising "sign in with ChatGPT" do. It
is out because the working part is Dango presenting itself as OpenAI's own client
to reach an endpoint reserved for that client, which is the access control
separating a subscription from the API. Running the vendor's CLI gets the same
result through the door the vendor left open, and does not put the user's account
behind our decision.

### Commands come from configuration, and the extension is rebuilt on reload

`Extension::manifest()` returns `&Manifest`, so a manifest behind a lock cannot
be handed out. Rather than change that signature for every extension, the AI
extension is immutable per configuration: `lib.rs` builds an instance from the
current config, and on a config reload builds a new one and replaces it.

`ExtensionHost` gains one method:

```rust
pub fn replace(&mut self, extension: Arc<dyn Extension>) -> Result<LoadReport, HostError>
```

which deactivates the old instance (unregistering its commands and stopping its
services), drops it, inserts the new one, and activates it when the enabled store
says enabled. `apply_config_reload` calls it before it rebuilds the search
candidates and the hotkey table, both of which then pick up the new command set
with no further work: hotkeys already come from the config file by command id.

Alternatives considered: putting the manifest behind an `RwLock` and returning a
clone (touches every extension for one caller); giving the registry a mutation
API (leaks one extension's internals into the shared registry).

### A command is a prompt template, and the template decides the input

There is no separate "input source" setting. A prompt that refers to
`{{ selection }}` reads the selection; one that refers to `{{ clipboard }}` reads
the clipboard; any other name becomes an argument the user is asked for, through
the same form flow the snippets extension uses. Reading the selection costs a
clipboard round trip on Windows, so it is read only when the prompt refers to it,
which is the rule `SnippetsExtension::values` already follows.

This is why Translate needs no special case: its prompt uses `{{ language }}`,
so the form appears, and the same mechanism lets a user-written command ask for
anything.

### Streaming: coalesced whole trees into the existing Detail view

Each fragment appends to a buffer; a tree is pushed when at least 50ms has passed
since the last push, plus one final tree when the answer completes. This keeps
the protocol's "thirty trees a second renders without flicker" budget comfortably
and stops a fast local model from flooding the channel.

The view is `Detail { markdown, loading }`: `loading` is true while the request
runs, and the text is the buffer. The detail view renders text verbatim today, so
markdown arrives as plain text; rendering it properly is a separate change and a
non-goal here.

`Command::invoke` runs on a `BuiltinHost` thread, which is synchronous, while
`genai` is async. The extension captures Tauri's tokio handle at construction
(`tauri::async_runtime::handle()`) and drives the stream with `block_on` on that
thread. Calling `tokio::spawn` from that thread without a handle would panic with
no reactor, which a release build turns into a silent abort; the captured handle
avoids that trap entirely.

### Cancelling actually stops the request

Superseding already hides a stale invocation's output, but an abandoned request
should also stop costing money and tokens. `InvocationContext` gains
`is_cancelled()`, backed by a flag the `Invoker` trips when an invocation is
superseded or cancelled, and the streaming loop checks it between fragments and
drops the stream, which ends the HTTP request.

A new `cancel_invocation` Tauri command trips the flag from the frontend: the
existing `dismiss` path calls it when the launcher hides, and popping a streaming
view with Escape calls it too.

### Configuration shape

The AI branch follows the `apps` precedent: a typed branch on the extension entry
that only one extension reads, and the existing per-command settings for the
parts that are not AI-specific.

```jsonc
"extensions": {
  "dango.ai": {
    "preferences": {
      "provider": "ollama",     // default provider for commands that name none
      "thinking": "off"         // default thinking level
    },
    "providers": {
      "anthropic": { "kind": "anthropic", "model": "claude-sonnet-5" },
      "ollama": { "kind": "ollama", "baseUrl": "http://localhost:11434", "model": "qwen3:8b" },
      "openrouter": { "kind": "openai", "baseUrl": "https://openrouter.ai/api/v1", "model": "z-ai/glm-4.6" }
    },
    "commands": {
      // Override a shipped command: its own model, and a hotkey.
      "improve-writing": { "provider": "anthropic", "thinking": "low", "hotkey": "hyper+i" },
      // Turn one off.
      "make-longer": { "enabled": false },
      // Add one. Title and prompt make it a command of its own.
      "review-rust": {
        "title": "Review Rust",
        "prompt": "Review this Rust for correctness and idiom. Answer with the issues only.\n\n{{ selection }}",
        "model": "claude-opus-5",
        "output": "view",
        "alias": "rr"
      }
    }
  }
}
```

`providers` is a new typed branch on the config's `Extension` struct, beside
`apps`. The AI-specific command fields (`title`, `prompt`, `provider`, `model`,
`thinking`, `output`, `enabled`) are optional fields added to the existing
`CommandSettings`, so `hotkey` and `alias` keep working through the M5 path with
no duplicate tree. `kind` is one of `anthropic`, `openai`, or `ollama`;
`thinking` is one of `off`, `low`, `medium`, `high`; `output` is one of `view`,
`paste`, `copy`.

`auth.json`, beside `config.json`, keyed by the provider's name in the config:

```json
{
  "anthropic": "sk-ant-...",
  "openrouter": "sk-or-..."
}
```

The shipped commands are Rust constants with the same fields, merged with the
config: a matching id overrides the fields it names, `enabled: false` drops it,
and an id with a title and a prompt that matches nothing shipped is a new
command. A config entry with neither a title nor a prompt and no shipped
counterpart is reported as a problem and skipped.

The defaults (`provider`, `thinking`) are declared preferences of kind string, so
they are read through the existing `Preferences` path and a future preferences
window gets them for free.

### Interface surfaces and primitives

No new surface. The streaming answer is the existing detail view, the argument
form is `FormFields`, the result actions are the existing action panel, and root
search is unchanged. Nothing is hand-rolled and no primitive is patched.

### Interface copy

Every new string, written against `openspec/specs/interface-copy/spec.md`.

Extension name: **AI**.

Command titles, Title Case because they sit in root search beside application
names:

- "Improve Writing"
- "Fix Spelling and Grammar"
- "Make Shorter"
- "Make Longer"
- "Make Simpler"
- "Make Professional"
- "Summarize"
- "Explain This"
- "Translate"

While a request runs, the detail view shows, until the first text arrives:

- "Working…"

Action labels on the result view, matching the labels the clipboard and snippets
extensions already use:

- "Paste"
- "Copy"

Failures, each one or two sentences saying what did not happen and what to do.
`{provider}` is the name the user gave the provider in their configuration:

- No provider configured: "No AI provider is set up yet. Add one to your config
  file."
- Command names a provider that is not configured: "This command uses a provider
  that isn't set up. Check your config file."
- Providers exist but none is the default: "No default AI provider is set. Add
  one to your config file." (Added while implementing: with one provider it is
  the default, so this only appears once a second one is added.)
- No model named anywhere: "No model is set for {provider}. Add one to your
  config file." (Added while implementing: a provider entry with no model and a
  command that names none has to say which of the two to fill in.)
- Key missing: "{provider} needs a key. Add one to auth.json in your config
  folder."
- Key rejected: "{provider} didn't accept your key. Check it in auth.json."
- Model unknown: "{provider} doesn't have that model. Check the model name in
  your config file."
- Hosted provider unreachable: "{provider} couldn't be reached. Check your
  connection and try again."
- Local provider unreachable: "The local model couldn't be reached. Check that
  your model server is running."
- Rate limited: "{provider} is busy right now. Try again in a moment."
- Timed out: "The model didn't answer in time. Try again."
- A command provider that is not installed: "Dango couldn't run {command}. Check
  that it's installed and on your PATH."
- A command provider that fails: "{provider} gave no answer. See dango.log for
  what it printed."
- Empty answer: "The model returned no text. Try again."
- Nothing selected: "Select some text first, then run this command."
- Prompt cannot be read: "This command's prompt can't be read. Check your config
  file."

Paste failures keep the messages the text plumbing already produces, including
the macOS Accessibility one, so there is nothing new to write for them.

## Risks / Trade-offs

- **A plain-text key can be committed by accident.** → It lives in its own file,
  the documentation says to keep `auth.json` out of git, and the example file
  ships with a placeholder. Nothing Dango writes ever contains a key.
- **`genai` is pre-1.0 and moving.** → Pin the exact minor, sit behind the
  `Completions` trait, and keep the mapping of its errors to Dango's messages in
  one place so an upgrade has one blast radius.
- **Provider errors are strings, and mapping them to friendly messages is
  guesswork.** → Map on the HTTP status where there is one (401, 404, 429,
  timeout) and fall back to a generic "{provider} couldn't be reached" with the
  detail in the log. Wrong mapping shows a slightly off message, never a raw
  error.
- **Rebuilding the extension on every config reload drops and restarts it.** →
  The AI extension has no services and no root provider, so a rebuild costs a
  registry re-registration and nothing else. A config reload that does not touch
  the AI branch still rebuilds; that is cheap enough to not be worth detecting.
- **A running request survives a reload.** → It holds its own resolved request
  and finishes or is cancelled on its own terms. The new instance only affects
  the next invocation.
- **Windows leaves the key file's permissions to the profile.** → Documented in
  the spec rather than papered over. A per-user profile directory is already the
  boundary that matters on that platform.
- **The detail view shows markdown as plain text**, so an answer with headings
  looks like literal `##`. → Accepted for this change, and the prompts ask for
  plain prose.
