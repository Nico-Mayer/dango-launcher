## ADDED Requirements

### Requirement: Directories to search for a command provider are configurable

The AI extension SHALL accept a list of directories in which to look for a
command provider's program. The list SHALL be given either once for both
platforms or separately for macOS and Windows; with the separate form, only the
list for the running platform SHALL apply, and a platform without a list SHALL
have none. A leading `~` in an entry SHALL stand for the user's home directory.

A command named without a directory SHALL be looked for in these directories in
the order given, then on Dango's own PATH. A command given as a path SHALL be run
as given. The program SHALL run with the configured directories placed before
Dango's PATH in its environment, so a program it starts in turn is found the same
way.

On Windows, a command named without an extension SHALL also be found as a
`.exe`, `.cmd`, or `.bat` file, as a shell finds it.

Changes to the list SHALL apply to the next command run, without a restart.

#### Scenario: A program outside Dango's PATH is found

- **WHEN** a provider names a program by its bare name, the program is in a
  configured directory, and that directory is not on Dango's PATH
- **THEN** a command using the provider runs the program and shows its answer

#### Scenario: The first configured directory wins

- **WHEN** two configured directories both contain the named program
- **THEN** the one in the directory listed first is run

#### Scenario: Dango's own PATH still applies

- **WHEN** no directory is configured, or none contains the program, and it is
  on Dango's PATH
- **THEN** the program is found and run as before

#### Scenario: Only this platform's list applies on macOS

- **WHEN** the list is given per platform and Dango runs on macOS
- **THEN** only the `macos` directories are searched
- **AND** the `windows` directories are ignored

#### Scenario: Only this platform's list applies on Windows

- **WHEN** the list is given per platform and Dango runs on Windows
- **THEN** only the `windows` directories are searched
- **AND** the `macos` directories are ignored

#### Scenario: A home-relative entry

- **WHEN** an entry starts with `~`
- **THEN** it is searched as a directory under the user's home directory

#### Scenario: The program's own children see the directories

- **WHEN** a provider's program is a wrapper that starts another program found
  only in a configured directory
- **THEN** that program is found and the answer arrives

#### Scenario: A command script is found on Windows

- **WHEN** Dango runs on Windows and the provider names `claude`, and the
  configured directory holds `claude.cmd` but no `claude.exe`
- **THEN** `claude.cmd` is run

#### Scenario: A program found nowhere

- **WHEN** the named program is in no configured directory and not on Dango's
  PATH
- **THEN** the failure says which command could not be run, as for a program
  that is not installed

#### Scenario: An edited list applies at once

- **WHEN** the user adds a directory to the list while Dango runs, then runs a
  command whose program is in that directory
- **THEN** the program is found without restarting Dango
