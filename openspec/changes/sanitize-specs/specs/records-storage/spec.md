## MODIFIED Requirements

### Requirement: Authored records live as text in the config directory

Snippets and quicklinks SHALL be stored as text files in the config directory,
`snippets.json` and `quicklinks.json` under `~/.config/dango/`, honouring
`$DANGO_CONFIG_DIR`. Each file SHALL be an array of records, and each record SHALL
carry a name and a body (the template for a snippet, the URL for a quicklink) and
a stable identifier.

Machine-local data, the clipboard history and usage ranking, SHALL NOT be
written to these files; it stays in the local database.

#### Scenario: A created record is written to its file

- **WHEN** the user creates a snippet
- **THEN** it appears as an entry in `snippets.json`
- **AND** the file is plain text that can be committed to git

#### Scenario: Machine-local data is not in the files

- **WHEN** the clipboard history records an entry
- **THEN** nothing is written to `snippets.json` or `quicklinks.json`

