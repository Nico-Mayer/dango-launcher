# Spec Delta

## ADDED Requirements

### Requirement: A missing quicklinks file starts with a web search

When the quicklinks extension is enabled and `quicklinks.toml` does not exist
at startup, Dango SHALL create it with one entry: a DuckDuckGo quicklink marked
as a web search (see quicklinks). Dango SHALL NOT add the entry to an existing
file, nor create the file when the quicklinks extension is disabled.

#### Scenario: First start

- **WHEN** Dango starts and `quicklinks.toml` does not exist
- **THEN** the file exists with its `#:schema` header and a `[duckduckgo]`
  entry with `name = "DuckDuckGo"`, a DuckDuckGo search URL that uses `query`,
  and `fallback = true`

#### Scenario: The user deleted the entry

- **WHEN** the user removed the `[duckduckgo]` entry from `quicklinks.toml`
  and Dango starts again
- **THEN** the file is left as it is and DuckDuckGo is not offered

#### Scenario: The user deleted the file

- **WHEN** the user deleted `quicklinks.toml` and Dango starts again
- **THEN** the file is created again with the DuckDuckGo entry

#### Scenario: The override directory

- **WHEN** `$DANGO_CONFIG_DIR` is set and has no `quicklinks.toml`
- **THEN** the file is created in that directory

#### Scenario: Quicklinks are turned off

- **WHEN** the quicklinks extension is disabled and `quicklinks.toml` does not exist
- **THEN** Dango does not create the file
