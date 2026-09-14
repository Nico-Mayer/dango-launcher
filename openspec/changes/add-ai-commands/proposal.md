## Why

**M6 - ai**, the whole milestone in one change. Everything M6 needs already
exists: M3 built selection capture, paste, and the template engine; M5 built the
config file and per-command hotkeys. What is missing is the piece in the middle
that takes selected text, sends it to a model, and streams the answer back.

The daily job is text transformation, not conversation: select a paragraph, press
a hotkey, get it rewritten. That is the Raycast Ollama extension's shape (fix
spelling, improve writing, make shorter, change tone) and it is what the author
wants. So the milestone ships as one extension with a set of prompt commands, and
a user writing a new prompt in `config.json` gets a command exactly like the
built-in ones.

Local models matter as much as hosted ones. An Ollama model on the same machine
needs no key, costs nothing, and keeps the text on the machine, so it is a first
class provider rather than an afterthought.

## What Changes

- A new built-in extension, `dango.ai`, contributing a set of one-shot text
  transform commands: Improve Writing, Fix Spelling and Grammar, Make Shorter,
  Make Longer, Make Simpler, Make Professional, Summarize, Explain This, and
  Translate.
- Every command is a prompt template rendered by the existing engine, so
  `{{ selection }}`, `{{ clipboard }}`, and arguments already work. A prompt with
  an argument (Translate asks for a language) shows the form the other extensions
  already use.
- Commands are data, not code. The shipped set is a built-in default; the user
  adds, overrides, or removes commands in `config.json` under
  `extensions."dango.ai".commands`, and each command may name its own provider,
  model, thinking level, and what happens to the answer. Command hotkeys and
  aliases work through the M5 path with no new mechanism.
- Providers are configured under `extensions."dango.ai".providers`: Anthropic,
  any OpenAI-compatible endpoint (OpenRouter, LM Studio, vLLM, and most others),
  and Ollama on the local machine. A provider names a base URL where it has one,
  and local providers need no credential at all.
- A fourth kind, `cli`, makes any program on the machine a provider: one that
  takes a prompt and prints an answer. `pi`, `claude -p`, `codex exec`, and
  anything else with that shape. It is how a ChatGPT or Claude subscription pays
  for a transform instead of API credits, through the client the vendor ships
  and the user has already signed in to. Like Ollama, it needs no key.
- API keys live in `auth.json` in the config directory, in plain text, keyed by
  provider. **This supersedes the project's "secrets go in the OS keychain"
  constraint**, deliberately: one hobby user, two machines, and a file that can
  be copied is worth more right now than a keychain round trip. The file is
  written with owner-only permissions where the platform allows it, and it is
  never written into `config.json` or a log.
- Inference runs in Rust and streams. The answer replaces the view tree as it
  arrives, using the existing full-tree-replace protocol, and Escape abandons a
  running request.
- When a command finishes, its `output` decides what happens: show the answer,
  paste it into the application the user came from (replacing the selection they
  had), or copy it. The result view always offers Paste and Copy as actions.
- The off-the-shelf choice is the `genai` crate: native Anthropic, an
  OpenAI-compatible adapter, and Ollama in one client with streaming and custom
  endpoints. A thin internal provider trait keeps it swappable. The design
  records what else was considered.

## Capabilities

### New Capabilities

- `ai-providers`: what a provider is, how one is configured, where the key lives
  and how it is read, how a local provider differs from a hosted one, how a
  model and a thinking level are chosen per command, and what the user sees when
  a key is missing, a model is wrong, or an endpoint cannot be reached.
- `ai-commands`: the `dango.ai` extension. The shipped command set, how a
  command is defined in configuration, how its prompt is rendered from the
  selection and arguments, how the answer streams into a view, what the output
  setting does, how a running request is abandoned, and the latency the user
  should see.

### Modified Capabilities

- `extension-model`: an extension's declared commands may now come from
  configuration rather than only from a fixed manifest, and they change while the
  app runs. A requirement is added for commands sourced from configuration being
  registered, re-registered, and unregistered live.
- `configuration`: the file is now also where commands themselves are defined,
  not just their hotkeys and aliases. The source-of-truth requirement gains that,
  plus a scenario for a command added by hand appearing without a restart.

## Impact

- **Affected code**: a new `src-tauri/src/extensions/ai/` module (extension,
  provider trait, adapters, auth file, streaming command); `src-tauri/src/config/`
  (the `dango.ai` branch: providers, command definitions, defaults);
  `src-tauri/src/extension/mod.rs` (replacing a live extension so its commands can
  be rebuilt on a config reload); `src-tauri/src/lib.rs` (wiring, the auth file,
  and the rebuild on reload).
- **Dependencies**: `genai` for the providers and streaming, and whatever it
  pulls in for HTTP and TLS. No new frontend dependency.
- **Config directory**: gains `auth.json` beside `config.json`, `snippets.json`,
  and `quicklinks.json`. Unlike the others it must not be committed, which the
  documentation says plainly.
- **Documentation**: `docs/config.example.jsonc` and `docs/config.schema.json`
  gain the `dango.ai` branch; a new `docs/auth.example.json` shows the key file.
- **Project constraints**: `openspec/config.yaml` says secrets go in the OS
  keychain. This change replaces that with the plain file and amends the
  constraint so the recorded context stays true.
- **View protocol**: unchanged. Streaming is repeated whole trees, which the
  protocol already requires.
- **Platforms**: Windows and macOS, both fully. Paste and selection come from the
  M3 plumbing, so the macOS Accessibility permission applies exactly as it does
  to snippets. Linux is out of scope for the project.

## Non-goals

- **No chat.** One prompt, one answer, no conversation, no history, no follow-up
  turn. A chat interface is a different product and is deferred in the roadmap.
- **No keychain.** Plain `auth.json` now; the provider trait reads a key through
  one function, so a keychain can replace it later without touching a command.
- **No re-implementing a vendor's first-party client.** A subscription is reached
  by running the CLI the vendor ships and the user has signed in to. Dango does
  not carry another client's identity or its tokens to reach an endpoint meant
  for that client, however well it would work.
- **No model picker or settings UI.** Provider and model are config, like
  everything else since M5. The preferences window is M7.
- **No tools, no function calling, no retrieval, no vision.** Text in, text out.
- **No markdown rendering.** The detail view shows the answer as text, which is
  what it does today. Rendering markdown is its own change.
- **No cost, token, or usage accounting.**
- **No automatic model discovery.** A model is named in the config; Dango does
  not list what an endpoint offers.
- **No Linux.**
