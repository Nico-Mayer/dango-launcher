# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Development setup

Tools are pinned in `mise.toml`. After cloning:

```sh
mise install
hk install --mise
```

The second command registers git hooks for this clone. Committing runs
`cargo fmt` on the staged Rust files and stages the result. Pushing runs
`cargo clippy --all-targets -- -D warnings` and `svelte-check`, the same checks
CI runs, and aborts the push if either fails. `hk check` and `hk fix` run the
same steps on demand. Use `git push --no-verify` to skip the push checks once.

## Configuration

Dango reads `~/.config/dango/config.json` (or `$DANGO_CONFIG_DIR`). Every
setting is optional and documented in `docs/config.example.jsonc`.

API keys for the AI commands live beside it in `auth.json`, in plain text, one
entry per provider. See `docs/auth.example.json`. **Keep `auth.json` out of
git.** The config file is meant to be committed and shared between machines;
the key file is not.
