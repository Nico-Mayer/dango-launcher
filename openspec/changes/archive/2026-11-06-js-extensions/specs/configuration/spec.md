# Spec Delta

## MODIFIED Requirements

### Requirement: The top level holds app tables and extension tables

The top level of the file SHALL hold `version`, the app tables `launcher` and
`hyperkey`, one table per built-in extension, and the reserved `extensions`
table. App tables configure Dango itself rather than a command. `extensions`
is kept for extensions that are not built in. Its table for one extension,
`extensions.<id>`, holds the enabled state, the per-command entries with
aliases and hotkeys, and any other key as that extension's preferences (see
`js-extensions`). The schema SHALL accept any preference key there. No
built-in extension's table SHALL be named `version`, `launcher`, `hyperkey`,
or `extensions`.

#### Scenario: Every built-in has its own table

- **WHEN** the settings reference is generated
- **THEN** each built-in extension is listed under a top-level table named by
  its id without `dango.`

#### Scenario: No extension takes a reserved name

- **WHEN** the built-in extensions are registered
- **THEN** none of their table names is `version`, `launcher`, `hyperkey`, or
  `extensions`

#### Scenario: A preference key is valid

- **WHEN** the file sets `user = "nico"` in `[extensions.gh-prs]`
- **THEN** the file loads without a problem and the value is a preference of
  `gh-prs`
