# Spec Delta

## MODIFIED Requirements

### Requirement: Authored records live as text in the config directory

Snippets and quicklinks SHALL be stored in `snippets.toml` and
`quicklinks.toml`, shared files under the contract in `config-files`. Each
record SHALL be a table keyed by its id, holding `name` and its body,
`template` for a snippet and `url` for a quicklink, and its optional fields:
`keyword` for a snippet, `browser` for a quicklink.

Machine-local data, the clipboard history and usage ranking, SHALL NOT be
written to these files; it stays in the local database.

#### Scenario: A created record is written to its file

- **WHEN** the user creates a snippet named "Signature"
- **THEN** `snippets.toml` holds a `[signature]` table with that name and its
  template
- **AND** the file is plain text that can be committed to git

#### Scenario: A multi-line template stays readable

- **WHEN** the user creates a snippet whose template has several lines
- **THEN** the file holds the template on several lines, not with `\n` escapes

#### Scenario: Machine-local data is not in the files

- **WHEN** the clipboard history records an entry
- **THEN** nothing is written to `snippets.toml` or `quicklinks.toml`

### Requirement: The files are hand-editable

A user SHALL be able to add, change, or remove a record by editing the file
directly, under an id of their choosing. A record created in the launcher
SHALL get an id made from its name: lowercase kebab-case, letters folded to
ASCII, never `version`, with a number appended when the id is taken. A record
SHALL keep its id when renamed.

#### Scenario: A hand-added record appears

- **WHEN** the user adds a `[brb]` table with a name and a template to
  `snippets.toml` by hand
- **THEN** the snippet is available in the launcher with id `brb`

#### Scenario: An id is made from the name

- **WHEN** the user creates a snippet named "Café menu"
- **THEN** it is stored under `[cafe-menu]`

#### Scenario: A taken id gets a number

- **WHEN** a quicklink `github` exists and the user creates another quicklink
  named "GitHub"
- **THEN** the new one is stored under `[github-2]`

#### Scenario: A rename keeps the id

- **WHEN** the user renames the snippet `signature` to "Work signature"
- **THEN** it is still stored under `[signature]` with the new name

#### Scenario: A name without letters or digits

- **WHEN** the user creates a snippet named "!!!"
- **THEN** it is stored under `[snippet]`, or `[snippet-2]` when that is taken

### Requirement: Removing a record removes its entry

Removing a record through the launcher SHALL delete its table from the file
rather than marking it deleted, and it SHALL stay gone across a restart.

#### Scenario: Remove deletes the entry

- **WHEN** the user removes a quicklink from the launcher
- **THEN** its table is gone from `quicklinks.toml`
- **AND** it does not return after a restart

## REMOVED Requirements

### Requirement: Edits apply live

**Reason**: Holds for every watched user file; now in `config-files`, "Edits
apply live".

**Migration**: None. Record files behave the same.

### Requirement: A bad records file is reported, not fatal

**Reason**: Holds for every user file; now in `config-files`, "A bad file is
reported, never fatal".

**Migration**: None.
