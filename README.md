# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Configuration

Dango reads `~/.config/dango/config.json` (or `$DANGO_CONFIG_DIR`). Every
setting is optional and documented in `docs/config.example.jsonc`.

API keys for the AI commands live beside it in `auth.json`, in plain text, one
entry per provider. See `docs/auth.example.json`. **Keep `auth.json` out of
git.** The config file is meant to be committed and shared between machines;
the key file is not.
