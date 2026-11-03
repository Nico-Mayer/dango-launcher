# Spec Delta

## MODIFIED Requirements

### Requirement: Terminology is consistent

One thing SHALL have one name everywhere it appears. The established names are:
"snippet", "quicklink", "clipboard history", "entry" for one item of the
history, "keyword" for a snippet's expansion trigger, "hotkey" for a global
chord, "config file" for `config.toml`, "the launcher" for Dango's window, and
"the previous app" for the application the user was in before summoning
Dango. A platform component SHALL be called what the platform calls it.

#### Scenario: A history item is an entry

- **WHEN** clipboard history text refers to one item
- **THEN** it says "entry", not "item", "record", or "clip"

#### Scenario: The application behind the launcher

- **WHEN** a message refers to the application the user came from
- **THEN** it says "the previous app", not "active app", "target", or
  "foreground window"

