# Spec Delta

## MODIFIED Requirements

### Requirement: Every user file is TOML in the config directory

Every file the user reads or edits SHALL be a TOML file in the config
directory described in `configuration`, honouring `$DANGO_CONFIG_DIR`. The
files SHALL be `config.toml`, `snippets.toml`, `quicklinks.toml`, and
`auth.toml`. Dango SHALL NOT read any other file there as user data. The
`extensions` folder holds extension code, which `js-extensions` describes, and
is not user data: the rules of this specification SHALL NOT apply to it.

#### Scenario: The four files

- **WHEN** Dango starts with all four files in the config directory
- **THEN** it reads settings from `config.toml`, snippets from
  `snippets.toml`, quicklinks from `quicklinks.toml`, and keys from
  `auth.toml`

#### Scenario: The override relocates every file

- **WHEN** `$DANGO_CONFIG_DIR` is set
- **THEN** all four files are read from that directory

#### Scenario: Extension files are not user files

- **WHEN** the `extensions` folder holds `manifest.json` and `main.js` files
- **THEN** Dango neither requires a schema for them nor writes a `#:schema`
  header into them
