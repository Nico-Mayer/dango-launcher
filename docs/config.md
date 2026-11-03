# Dango configuration

Every setting in config.toml. Generated from the code; do not edit by hand.

## Top level

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `ai` | object |  |  | Settings for the AI extension. |
| `applications` | object |  |  | Settings for the Applications extension. |
| `clipboard` | object |  |  | Settings for the Clipboard History extension. |
| `extensions` | map of [Extension table](#extension-table) |  |  | Extensions that are not built in, keyed by extension id. Holds only what Dango itself reads for them: enabled state, aliases, and hotkeys. |
| `games` | object |  |  | Settings for the Games extension. |
| `hyperkey` | [Hyperkey](#hyperkey) |  |  | CapsLock, remapped to act as the hyper modifier system-wide. Present means on; leave it out to turn it off. On macOS it uses F18, so a hotkey with `f18` does not work while it is on. |
| `launcher` | [Launcher](#launcher) |  |  | Settings of the launcher itself. |
| `permissions` | object |  |  | Settings for the Permissions extension. |
| `quicklinks` | object |  |  | Settings for the Quicklinks extension. |
| `selection` | [Selection](#selection) |  |  | How Dango reads the text you have selected. |
| `snippets` | object |  |  | Settings for the Snippets extension. |
| `system` | object |  |  | Settings for the System extension. |
| `updates` | object |  |  | Settings for the Updates extension. |
| `version` | integer |  |  | The config schema version. Leave it at 1. |
| `window-management` | object |  |  | Settings for the Window Management extension. |

## `ai`

Settings for the AI extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | map of [AI command settings](#ai-command-settings) |  |  | Per-command settings, keyed by command id. A command id the extension does not ship defines a new command. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `provider` | string |  |  | The provider a command uses when it names none. |
| `providers` | map of [Provider settings](#provider-settings) |  |  | Model services, keyed by the name commands refer to them by. That name is also the key its API key is filed under in auth.json. |
| `thinking` | string | `off` | `off`, `low`, `medium`, `high` | How hard the model thinks when a command does not say. |

## `ai.commands`

Per-command settings, keyed by command id. A command id the extension does not ship defines a new command.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `explain` | [AI command settings](#ai-command-settings) |  |  | Settings for the explain command. |
| `fix-spelling` | [AI command settings](#ai-command-settings) |  |  | Settings for the fix-spelling command. |
| `improve-writing` | [AI command settings](#ai-command-settings) |  |  | Settings for the improve-writing command. |
| `make-longer` | [AI command settings](#ai-command-settings) |  |  | Settings for the make-longer command. |
| `make-professional` | [AI command settings](#ai-command-settings) |  |  | Settings for the make-professional command. |
| `make-shorter` | [AI command settings](#ai-command-settings) |  |  | Settings for the make-shorter command. |
| `make-simpler` | [AI command settings](#ai-command-settings) |  |  | Settings for the make-simpler command. |
| `summarize` | [AI command settings](#ai-command-settings) |  |  | Settings for the summarize command. |
| `translate` | [AI command settings](#ai-command-settings) |  |  | Settings for the translate command. |

## `applications`

Settings for the Applications extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `hotkeys` | map of [Application settings](#application-settings) |  |  | Applications bound to global hotkeys, keyed by the name Dango shows for the application in root search. |

## `clipboard`

Settings for the Clipboard History extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `entries` | integer | `500` |  | How many entries the history keeps. |
| `entry-megabytes` | integer | `16` |  | The largest single entry kept, in megabytes. |
| `excluded-applications` | array | `1Password`, `Bitwarden`, `Dashlane`, `Enpass`, `KeePassXC`, `Keeper`, `LastPass`, `NordPass`, `Proton Pass`, `Secrets`, `Strongbox` |  | Applications whose copies are never recorded, matched ignoring case. |
| `primary-action` | string | `paste` | `paste`, `copy` | What Enter does to an entry: paste it into the previous app, or only copy it. |
| `total-megabytes` | integer | `256` |  | The most the whole history may use, in megabytes. |

## `clipboard.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `history` | [Command settings](#command-settings) |  |  | Settings for the history command. |

## `games`

Settings for the Games extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `games.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `search` | [Command settings](#command-settings) |  |  | Settings for the search command. |

## `permissions`

Settings for the Permissions extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `quicklinks`

Settings for the Quicklinks extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `favicons` | boolean | `true` |  | Show each quicklink with its site's icon. Off stops Dango fetching from those sites. |

## `quicklinks.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `create` | [Command settings](#command-settings) |  |  | Settings for the create command. |
| `search` | [Command settings](#command-settings) |  |  | Settings for the search command. |

## `snippets`

Settings for the Snippets extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `snippets.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `create` | [Command settings](#command-settings) |  |  | Settings for the create command. |
| `search` | [Command settings](#command-settings) |  |  | Settings for the search command. |

## `system`

Settings for the System extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `system.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `empty-trash` | [Command settings](#command-settings) |  |  | Settings for the empty-trash command. |
| `lock` | [Command settings](#command-settings) |  |  | Settings for the lock command. |
| `open-config` | [Command settings](#command-settings) |  |  | Settings for the open-config command. |
| `quit-application` | [Command settings](#command-settings) |  |  | Settings for the quit-application command. |
| `sleep` | [Command settings](#command-settings) |  |  | Settings for the sleep command. |

## `updates`

Settings for the Updates extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `updates.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `check` | [Command settings](#command-settings) |  |  | Settings for the check command. |

## `window-management`

Settings for the Window Management extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `window-management.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `almost-maximize` | [Command settings](#command-settings) |  |  | Settings for the almost-maximize command. |
| `bottom-half` | [Command settings](#command-settings) |  |  | Settings for the bottom-half command. |
| `bottom-left-quarter` | [Command settings](#command-settings) |  |  | Settings for the bottom-left-quarter command. |
| `bottom-right-quarter` | [Command settings](#command-settings) |  |  | Settings for the bottom-right-quarter command. |
| `center` | [Command settings](#command-settings) |  |  | Settings for the center command. |
| `center-half` | [Command settings](#command-settings) |  |  | Settings for the center-half command. |
| `center-third` | [Command settings](#command-settings) |  |  | Settings for the center-third command. |
| `left-half` | [Command settings](#command-settings) |  |  | Settings for the left-half command. |
| `left-third` | [Command settings](#command-settings) |  |  | Settings for the left-third command. |
| `make-larger` | [Command settings](#command-settings) |  |  | Settings for the make-larger command. |
| `make-smaller` | [Command settings](#command-settings) |  |  | Settings for the make-smaller command. |
| `maximize` | [Command settings](#command-settings) |  |  | Settings for the maximize command. |
| `next-display` | [Command settings](#command-settings) |  |  | Settings for the next-display command. |
| `reasonable-size` | [Command settings](#command-settings) |  |  | Settings for the reasonable-size command. |
| `right-half` | [Command settings](#command-settings) |  |  | Settings for the right-half command. |
| `right-third` | [Command settings](#command-settings) |  |  | Settings for the right-third command. |
| `top-half` | [Command settings](#command-settings) |  |  | Settings for the top-half command. |
| `top-left-quarter` | [Command settings](#command-settings) |  |  | Settings for the top-left-quarter command. |
| `top-right-quarter` | [Command settings](#command-settings) |  |  | Settings for the top-right-quarter command. |

## Extension table

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | map of [Command settings](#command-settings) |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## Hyperkey

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `shift` | boolean |  |  | Whether Shift is part of the combination. Default true; false emits Ctrl+Alt+Super, which leaves typed characters unshifted. |
| `tap` | string |  | `space`, `enter`, `return`, `tab`, `escape`, `esc`, `backspace`, `delete` and 78 more | A key to send on a quick, solitary tap of the hyperkey, such as `escape`. A hold, or a press with another key, still acts as the modifier. Leave it out for no tap action. |

## Launcher

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `autostart` | boolean |  |  | Start Dango when you log in, on the machine that reads this file. Default false. A development build ignores it. |
| `hotkey` | [Hotkey](#hotkey) |  |  | The chord that summons the launcher. Default alt+space, which is Option+Space on macOS. |
| `reset-after` | integer |  |  | Seconds the launcher keeps what you were doing after it is hidden by the hotkey or by losing focus. Pressing the hotkey again within this time continues where you left off. Default 60. 0 resets to root search at once. |

## Selection

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `excluded-applications` | string |  |  | Applications where Dango must never send the copy chord to read a selection, comma separated and matched ignoring case. For editors that bind Ctrl+C to something of their own. Dango says it cannot read the selection there instead. |

## Provider settings

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `base-url` | string |  |  | Where the service lives. Anthropic needs none; an OpenAI-compatible endpoint always does; Ollama defaults to the local one. |
| `kind` | string |  | `anthropic`, `openai`, `ollama`, `claude`, `pi` | The protocol a provider speaks. `openai` covers any OpenAI-compatible endpoint, such as OpenRouter, LM Studio, or vLLM. `claude` and `pi` run that CLI with the sign-in it already has. |
| `model` | string |  |  | The model used by a command that names this provider but no model. Optional for `claude` and `pi`, which then use their own default. |
| `path` | string |  |  | Absolute path to the program, for `claude` and `pi`. Without it Dango looks on PATH and in the usual install folders. |

## AI command settings

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `alias` | string |  |  | A short alias that matches this command in root search. |
| `enabled` | boolean |  |  | Whether the command is offered. Absent means yes; false removes a command the extension ships. |
| `hotkey` | [Hotkey](#hotkey) |  |  | A global hotkey that runs this command from anywhere. The command shows its view. |
| `model` | string |  |  | The model this command uses. Absent falls back to the provider's model. |
| `output` | [Output](#output) |  |  | What becomes of the answer: shown in the launcher, pasted into the previous app, or copied. Default view. |
| `prompt` | string |  |  | What the model is asked, as a template. Placeholders: {{ selection }}, {{ clipboard }}, and any other name, which the launcher asks for. |
| `provider` | string |  |  | The provider this command uses, by its name under providers. Absent falls back to the extension's default provider. |
| `thinking` | string |  | `off`, `low`, `medium`, `high` | How hard the model thinks before answering. Absent falls back to the extension's default level. |
| `title` | string |  |  | The title root search shows. Needed for a command the file defines from nothing. |

## Application settings

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `hotkey` | [Hotkey](#hotkey) |  |  | The chord that opens the application, or brings it to the front. |
| `name` | [Application name](#application-name) |  |  | The application's shown name when it differs from the key, either one string or one per platform. A side with no value means the entry is skipped on that platform. |

## Command settings

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `alias` | string |  |  | A short alias that matches this command in root search. |
| `enabled` | boolean |  |  | Whether the command is offered. Absent means yes; false removes a command the extension ships. |
| `hotkey` | [Hotkey](#hotkey) |  |  | A global hotkey that runs this command from anywhere. A no-view command runs without showing the launcher; a view command shows its view. |

## Hotkey

A hotkey: one chord that works on both platforms, such as `mod+shift+k` where `mod` is Cmd on macOS and Ctrl on Windows, or a chord per platform. The key is a physical position, so a non-US layout does not move it.

A chord is one or more modifiers joined with `+`, then one key, all lowercase. A function key from `f1` to `f20` can also stand alone.

- Modifiers: `mod`, `ctrl`, `control`, `alt`, `opt`, `option`, `shift`, `meta`, `cmd`, `command`, `super`, `win`, `windows`, `hyper`.
- Keys: a letter, a digit, or one of `space`, `enter`, `return`, `tab`, `escape`, `esc`, `backspace`, `delete`, `del`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`, `comma`, `period`, `dot`, `slash`, `backslash`, `semicolon`, `quote`, `backquote`, `grave`, `minus`, `equal`, `leftbracket`, `rightbracket`, `f1`, `f2`, `f3`, `f4`, `f5`, `f6`, `f7`, `f8`, `f9`, `f10`, `f11`, `f12`, `f13`, `f14`, `f15`, `f16`, `f17`, `f18`, `f19`, `f20`.

Or an object with a `macos` and a `windows` chord; a missing side means no binding there.

## Application name

An application's shown name: one string for both platforms, or one per platform, the same two shapes a hotkey may take.

One of:

- string
- [Per-platform value](#per-platform-value)

## Per-platform value

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `macos` | string |  |  | The value on macOS. |
| `windows` | string |  |  | The value on Windows. |
