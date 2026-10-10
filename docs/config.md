# Dango configuration

Every setting in config.json. Generated from the code; do not edit by hand.

## Top level

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `$schema` | string |  |  | Where your editor finds this schema. Dango writes it beside the config file, so `./config.schema.json` is the value to use. |
| `extensions` | object |  |  | Per-extension settings, keyed by extension id. |
| `hyperkey` | [Hyperkey](#hyperkey) |  |  | A physical key remapped to act as the hyper modifier system-wide. Present means on; leave it out to turn it off. |
| `launcher` | [Launcher](#launcher) |  |  | Settings of the launcher itself. |
| `selection` | [Selection](#selection) |  |  | How Dango reads the text you have selected. |
| `version` | integer |  |  | The config schema version. Leave it at 1. |

## `extensions`

Per-extension settings, keyed by extension id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `dango.ai` | object |  |  | Settings for the AI extension. |
| `dango.applications` | object |  |  | Settings for the Applications extension. |
| `dango.clipboard` | object |  |  | Settings for the Clipboard History extension. |
| `dango.quicklinks` | object |  |  | Settings for the Quicklinks extension. |
| `dango.snippets` | object |  |  | Settings for the Snippets extension. |
| `dango.system` | object |  |  | Settings for the System extension. |
| `dango.window-management` | object |  |  | Settings for the Window Management extension. |

## `extensions.dango.ai`

Settings for the AI extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | map of [AI command settings](#ai-command-settings) |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `preferences` | object |  |  | The extension's preference values, overriding its defaults. |
| `providers` | map of [Provider settings](#provider-settings) |  |  | Model services, keyed by the name commands refer to them by. That name is also the key its API key is filed under in auth.json. |

## `extensions.dango.ai.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `explain` | [AI command settings](#ai-command-settings) |  |  | Settings for Explain This. |
| `fix-spelling` | [AI command settings](#ai-command-settings) |  |  | Settings for Fix Spelling and Grammar. |
| `improve-writing` | [AI command settings](#ai-command-settings) |  |  | Settings for Improve Writing. |
| `make-longer` | [AI command settings](#ai-command-settings) |  |  | Settings for Make Longer. |
| `make-professional` | [AI command settings](#ai-command-settings) |  |  | Settings for Make Professional. |
| `make-shorter` | [AI command settings](#ai-command-settings) |  |  | Settings for Make Shorter. |
| `make-simpler` | [AI command settings](#ai-command-settings) |  |  | Settings for Make Simpler. |
| `summarize` | [AI command settings](#ai-command-settings) |  |  | Settings for Summarize. |
| `translate` | [AI command settings](#ai-command-settings) |  |  | Settings for Translate. |

## `extensions.dango.ai.preferences`

The extension's preference values, overriding its defaults.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `provider` | string |  |  | The provider a command uses when it names none. |
| `thinking` | string | `off` | `off`, `low`, `medium`, `high` | How hard the model thinks when a command does not say. |

## `extensions.dango.applications`

Settings for the Applications extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `apps` | map of [Application settings](#application-settings) |  |  | Applications bound to global hotkeys, keyed by the name Dango shows for the application in root search. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `extensions.dango.clipboard`

Settings for the Clipboard History extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `preferences` | object |  |  | The extension's preference values, overriding its defaults. |

## `extensions.dango.clipboard.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `history` | [Command settings](#command-settings) |  |  | Settings for Clipboard History. |

## `extensions.dango.clipboard.preferences`

The extension's preference values, overriding its defaults.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `entries` | number | `500` |  | How many entries the history keeps. |
| `entry-megabytes` | number | `16` |  | The largest single entry kept, in megabytes. |
| `excluded-applications` | string | `1Password, Bitwarden, Dashlane, Enpass, KeePassXC, Keeper, LastPass, NordPass, Proton Pass, Secrets, Strongbox` |  | Applications whose copies are never recorded, comma separated. |
| `primary-action` | string | `paste` | `paste`, `copy` | What Enter does to an entry: paste it into the previous app, or only copy it. |
| `total-megabytes` | number | `256` |  | The most the whole history may use, in megabytes. |

## `extensions.dango.quicklinks`

Settings for the Quicklinks extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |
| `preferences` | object |  |  | The extension's preference values, overriding its defaults. |

## `extensions.dango.quicklinks.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `create` | [Command settings](#command-settings) |  |  | Settings for Create Quicklink. |
| `search` | [Command settings](#command-settings) |  |  | Settings for Search Quicklinks. |

## `extensions.dango.quicklinks.preferences`

The extension's preference values, overriding its defaults.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `favicons` | boolean | `true` |  | Show each quicklink with its site's icon. Off stops Dango fetching from those sites. |

## `extensions.dango.snippets`

Settings for the Snippets extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `extensions.dango.snippets.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `create` | [Command settings](#command-settings) |  |  | Settings for Create Snippet. |
| `search` | [Command settings](#command-settings) |  |  | Settings for Search Snippets. |

## `extensions.dango.system`

Settings for the System extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `extensions.dango.system.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `empty-trash` | [Command settings](#command-settings) |  |  | Settings for Empty Recycle Bin. |
| `lock` | [Command settings](#command-settings) |  |  | Settings for Lock Screen. |
| `open-config` | [Command settings](#command-settings) |  |  | Settings for Open Config File. |
| `quit-application` | [Command settings](#command-settings) |  |  | Settings for Quit Application. |
| `sleep` | [Command settings](#command-settings) |  |  | Settings for Sleep. |

## `extensions.dango.window-management`

Settings for the Window Management extension.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `commands` | object |  |  | Per-command settings, keyed by command id. |
| `enabled` | boolean |  |  | Whether the extension runs. Absent means enabled. |

## `extensions.dango.window-management.commands`

Per-command settings, keyed by command id.

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `almost-maximize` | [Command settings](#command-settings) |  |  | Settings for Almost Maximize. |
| `bottom-half` | [Command settings](#command-settings) |  |  | Settings for Bottom Half. |
| `bottom-left-quarter` | [Command settings](#command-settings) |  |  | Settings for Bottom Left Quarter. |
| `bottom-right-quarter` | [Command settings](#command-settings) |  |  | Settings for Bottom Right Quarter. |
| `center` | [Command settings](#command-settings) |  |  | Settings for Center. |
| `center-half` | [Command settings](#command-settings) |  |  | Settings for Center Half. |
| `center-third` | [Command settings](#command-settings) |  |  | Settings for Center Third. |
| `left-half` | [Command settings](#command-settings) |  |  | Settings for Left Half. |
| `left-third` | [Command settings](#command-settings) |  |  | Settings for Left Third. |
| `make-larger` | [Command settings](#command-settings) |  |  | Settings for Make Larger. |
| `make-smaller` | [Command settings](#command-settings) |  |  | Settings for Make Smaller. |
| `maximize` | [Command settings](#command-settings) |  |  | Settings for Maximize. |
| `next-display` | [Command settings](#command-settings) |  |  | Settings for Move to Next Display. |
| `reasonable-size` | [Command settings](#command-settings) |  |  | Settings for Reasonable Size. |
| `right-half` | [Command settings](#command-settings) |  |  | Settings for Right Half. |
| `right-third` | [Command settings](#command-settings) |  |  | Settings for Right Third. |
| `top-half` | [Command settings](#command-settings) |  |  | Settings for Top Half. |
| `top-left-quarter` | [Command settings](#command-settings) |  |  | Settings for Top Left Quarter. |
| `top-right-quarter` | [Command settings](#command-settings) |  |  | Settings for Top Right Quarter. |

## Hyperkey

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `key` | string | `capslock` | `capslock`, `caps` | The physical key to remap. Defaults to capslock. |
| `shift` | boolean |  |  | Whether Shift is part of the combination. Default true; false emits Ctrl+Alt+Super, which leaves typed characters unshifted. |
| `tap` | string |  | `space`, `enter`, `return`, `tab`, `escape`, `esc`, `backspace`, `delete` and 70 more | A key to send on a quick, solitary tap of the hyperkey, such as `escape`. A hold, or a press with another key, still acts as the modifier. Leave it out for no tap action. |

## Launcher

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `autostart` | boolean |  |  | Start Dango when you log in, on the machine that reads this file. Default false. A development build ignores it. |
| `hotkey` | [Hotkey](#hotkey) |  |  | The chord that summons the launcher. Default alt+space, which is Option+Space on macOS. |

## Selection

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `excluded-applications` | string |  |  | Applications where Dango must never send the copy chord to read a selection, comma separated and matched ignoring case. For editors that bind Ctrl+C to something of their own. Dango says it cannot read the selection there instead. |

## Provider settings

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `args` | array |  |  | What to pass it. `{prompt}` and `{model}` are replaced where they appear; with no `{prompt}`, the prompt goes in on standard input. |
| `baseUrl` | string |  |  | Where the service lives. Anthropic needs none; an OpenAI-compatible endpoint always does; Ollama defaults to the local one. |
| `command` | string |  |  | The program to run, for a provider of the `cli` kind. |
| `kind` | string |  | `anthropic`, `openai`, `ollama`, `cli` | The protocol a provider speaks. `openai` covers any OpenAI-compatible endpoint, such as OpenRouter, LM Studio, or vLLM. `cli` runs a program on this machine that takes a prompt and prints an answer. |
| `model` | string |  |  | The model used by a command that names this provider but no model. |

## AI command settings

| Setting | Type | Default | Values | Description |
| --- | --- | --- | --- | --- |
| `alias` | string |  |  | A short alias that matches this command in root search. |
| `enabled` | boolean |  |  | Whether the command is offered. Absent means yes; false removes a command the extension ships. |
| `hotkey` | [Hotkey](#hotkey) |  |  | A global hotkey that runs this command from anywhere. A no-view command runs without showing the launcher; a view command shows its view. |
| `model` | string |  |  | The model this command uses. Absent falls back to the provider's model. |
| `output` | [Output](#output) |  |  | What becomes of the answer: shown in the launcher, pasted into the previous app, or copied. Default view. |
| `preferences` | object |  |  | Preference values scoped to this command. |
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
| `preferences` | object |  |  | Preference values scoped to this command. |

## Hotkey

A hotkey: one chord that works on both platforms, such as `mod+shift+k` where `mod` is Cmd on macOS and Ctrl on Windows, or a chord per platform. The key is a physical position, so a non-US layout does not move it.

A chord is one or more modifiers joined with `+`, then one key, all lowercase.

- Modifiers: `mod`, `ctrl`, `control`, `alt`, `opt`, `option`, `shift`, `meta`, `cmd`, `command`, `super`, `win`, `windows`, `hyper`.
- Keys: a letter, a digit, or one of `space`, `enter`, `return`, `tab`, `escape`, `esc`, `backspace`, `delete`, `del`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`, `comma`, `period`, `dot`, `slash`, `backslash`, `semicolon`, `quote`, `backquote`, `grave`, `minus`, `equal`, `leftbracket`, `rightbracket`, `f1`, `f2`, `f3`, `f4`, `f5`, `f6`, `f7`, `f8`, `f9`, `f10`, `f11`, `f12`.

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
