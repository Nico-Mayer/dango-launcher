# records-storage Specification

## Purpose

Storing the snippets and quicklinks the user authors as plain text files in the
config directory, so they travel in the user's dotfiles and git alongside the
config, can be edited by hand, and are never a binary blob that conflicts across
machines.

## Requirements

### Requirement: Authored records live as text in the config directory

Snippets and quicklinks SHALL be stored as text files in the config directory,
`snippets.json` and `quicklinks.json` under `~/.config/dango/`, honouring
`$DANGO_CONFIG_DIR`. Each file SHALL be an array of records, and each record SHALL
carry a name and a body (the template for a snippet, the URL for a quicklink) and
a stable identifier.

Machine-local data, the clipboard history, usage ranking, and window state, SHALL
NOT be written to these files; it stays in the local database.

#### Scenario: A created record is written to its file

- **WHEN** the user creates a snippet
- **THEN** it appears as an entry in `snippets.json`
- **AND** the file is plain text that can be committed to git

#### Scenario: Machine-local data is not in the files

- **WHEN** the clipboard history records an entry
- **THEN** nothing is written to `snippets.json` or `quicklinks.json`

### Requirement: The files are hand-editable

A user SHALL be able to add, change, or remove a record by editing the file
directly. A record MAY omit its identifier when hand-authored, and Dango SHALL
assign one the next time it writes the file, without changing the record's meaning.

#### Scenario: A hand-added record appears

- **WHEN** the user adds a record to `snippets.json` by hand, without an id
- **THEN** the snippet is available in the launcher
- **AND** Dango gives it an id when it next writes the file

### Requirement: Edits apply live

Dango SHALL watch the record files and reflect changes without a restart: a
record added, changed, or removed in a file SHALL show up in the launcher on the
next search, and Dango's own writes SHALL NOT be treated as external edits.

#### Scenario: Editing a record updates the launcher

- **WHEN** the user changes a snippet's body in `snippets.json` while Dango runs
- **THEN** using that snippet inserts the new body, without a restart

#### Scenario: Deleting an entry removes the record

- **WHEN** the user removes a record's entry from the file
- **THEN** that record no longer appears in the launcher

### Requirement: Removing a record removes its entry

Removing a record through the launcher SHALL delete its entry from the file
rather than marking it deleted, and it SHALL stay gone across a restart.

#### Scenario: Remove deletes the entry

- **WHEN** the user removes a quicklink from the launcher
- **THEN** its entry is gone from `quicklinks.json`
- **AND** it does not return after a restart

### Requirement: A bad records file is reported, not fatal

An unreadable or invalid record file SHALL NOT crash Dango or discard the user's
records. Dango SHALL keep the last set that loaded cleanly and surface the problem
where the user can find it, the same way a bad config file is handled.

#### Scenario: A malformed file keeps the last good records

- **WHEN** the user saves a `snippets.json` that does not parse
- **THEN** the snippets from the last good version are still available
- **AND** the error is surfaced rather than the records being lost
