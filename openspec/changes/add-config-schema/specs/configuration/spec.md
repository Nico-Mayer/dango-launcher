## MODIFIED Requirements

### Requirement: Configuration lives in one JSON file

Dango SHALL read its configuration from a single JSON file named `config.json` in
a fixed directory, `~/.config/dango/` on both platforms, and SHALL honour a
`$DANGO_CONFIG_DIR` environment override for that directory. The directory SHALL
be created if it does not exist.

The file SHALL be optional: with no file present, Dango runs entirely on its
built-in defaults. The schema Dango writes beside the file is not
configuration and SHALL NOT count as the file being present.

#### Scenario: No file means defaults

- **WHEN** Dango starts and no `config.json` exists
- **THEN** it runs normally with every setting at its built-in default
- **AND** it does not create a config file the user did not ask for

#### Scenario: The override relocates the directory

- **WHEN** `$DANGO_CONFIG_DIR` is set and contains a `config.json`
- **THEN** Dango reads configuration from there rather than `~/.config/dango/`
