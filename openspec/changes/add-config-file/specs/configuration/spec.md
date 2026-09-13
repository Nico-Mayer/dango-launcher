## Purpose

Configuring Dango through a single JSON file the user keeps in their dotfiles and
shares over git, rather than a settings interface: what the file controls, where
it lives, how it is loaded and reloaded live, and how a mistake in it is handled
without taking the app down.

## ADDED Requirements

### Requirement: Configuration lives in one JSON file

Dango SHALL read its configuration from a single JSON file named `config.json` in
a fixed directory, `~/.config/dango/` on both platforms, and SHALL honour a
`$DANGO_CONFIG_DIR` environment override for that directory. The directory SHALL
be created if it does not exist.

The file SHALL be optional: with no file present, Dango runs entirely on its
built-in defaults.

#### Scenario: No file means defaults

- **WHEN** Dango starts and no `config.json` exists
- **THEN** it runs normally with every setting at its built-in default
- **AND** it does not create a file the user did not ask for

#### Scenario: The override relocates the directory

- **WHEN** `$DANGO_CONFIG_DIR` is set and contains a `config.json`
- **THEN** Dango reads configuration from there rather than `~/.config/dango/`

### Requirement: The file is the source of truth for configuration

The configuration file SHALL be where the launcher hotkey, each extension's
enabled state, each extension's preference values, and command aliases come from.
A value present in the file SHALL take precedence over the built-in default; a
value absent from the file SHALL fall back to the default.

Configuration SHALL be addressed by extension id and, where it applies, command
id, so every setting has one stable path and new settings extend the same tree.

#### Scenario: A preference set in the file is used

- **WHEN** the file sets a preference for an extension and that extension reads it
- **THEN** the value from the file is returned rather than the declared default

#### Scenario: An extension disabled in the file is disabled

- **WHEN** the file marks an extension disabled and Dango starts
- **THEN** that extension's commands, root items, and services are not active

#### Scenario: An absent value falls back to the default

- **WHEN** the file does not mention a preference
- **THEN** reading it returns the extension's declared default

### Requirement: Configuration is versioned and forward compatible

The file SHALL carry a version. Keys the running version does not understand,
including settings for extensions that are not installed and settings from a
newer Dango, SHALL be preserved and ignored rather than treated as errors, so an
older Dango and a newer one can share one file.

#### Scenario: Unknown keys do not break loading

- **WHEN** the file contains a key Dango does not recognise
- **THEN** the rest of the configuration still loads and applies
- **AND** the unknown key is left intact for whatever does understand it

### Requirement: Edits apply live

Dango SHALL watch the configuration file and re-apply it when it changes, without
a restart: the launcher hotkey, enabled state, preferences, and aliases SHALL
reflect the new file. When Dango itself writes the file, that write SHALL NOT be
treated as an external edit to react to.

#### Scenario: Editing the file re-applies it

- **WHEN** the user edits and saves `config.json` while Dango is running
- **THEN** the changed settings take effect without restarting Dango

#### Scenario: The app's own write does not loop

- **WHEN** Dango writes the file itself
- **THEN** it does not re-apply its own write as though the user had edited it

### Requirement: A bad file never takes the app down

An unreadable or invalid configuration file SHALL NOT crash Dango or leave it
unconfigured. Dango SHALL keep running on the last configuration that loaded
cleanly, or on the built-in defaults if none has, and SHALL surface the problem
where the user can find it rather than failing silently.

#### Scenario: Invalid file keeps the last good configuration

- **WHEN** the user saves a `config.json` that does not parse
- **THEN** Dango keeps running on the previously loaded configuration
- **AND** the error is surfaced through the tray and written to a log

#### Scenario: Invalid file at startup falls back to defaults

- **WHEN** Dango starts and the existing `config.json` does not parse
- **THEN** Dango starts on its built-in defaults
- **AND** the error is surfaced rather than the app refusing to start

### Requirement: The file survives being written back

The loader SHALL represent the file so that writing it back preserves settings it
did not itself change, including keys it does not understand, so a future editor
of the file, whether a person or a settings interface, does not lose data by
saving.

#### Scenario: Writing back keeps unrelated settings

- **WHEN** one setting is changed and the file is written back
- **THEN** every other setting in the file, including any unrecognised key, is
  still present afterward
