# Spec Delta

## MODIFIED Requirements

### Requirement: Keys live in a plain file in the config directory

Keys for hosted providers SHALL be read from `auth.json` in the configuration
directory described in `openspec/specs/configuration`. The file SHALL map a
provider's name to its key. It SHALL be optional: with no file, only providers
needing no key work.

Dango SHALL only read this file; it SHALL NOT write keys into it. A key SHALL
NOT be written into `config.toml`, into the local database, into a log, or into
any message shown on screen. Where the platform expresses file permissions,
Dango SHALL keep the file readable only by its owner.

#### Scenario: A key is read for a hosted provider

- **WHEN** `auth.json` holds a key for a provider and a command uses that
  provider
- **THEN** the request is made with that key

#### Scenario: The key file is missing

- **WHEN** a command uses a hosted provider and no `auth.json` exists
- **THEN** the command fails with a message naming the provider and saying a key
  is needed
- **AND** the message names the file to put it in

#### Scenario: The key file cannot be read

- **WHEN** `auth.json` exists but does not parse and a command uses a hosted
  provider
- **THEN** the problem is written to the log
- **AND** the command fails with "auth.json couldn't be read. Check the file in
  your config folder."

#### Scenario: A local provider works with an unreadable key file

- **WHEN** `auth.json` exists but does not parse and a command uses a local
  provider
- **THEN** the command runs normally

#### Scenario: A key never appears anywhere else

- **WHEN** a request has been made with a key and the log, the configuration
  file, and every message shown are inspected
- **THEN** the key appears in none of them

#### Scenario: The key file is restricted to its owner on macOS

- **WHEN** `auth.json` is readable by users other than its owner and Dango reads
  it
- **THEN** Dango restricts it to its owner
- **AND** the key is still read

#### Scenario: The key file on Windows

- **WHEN** `auth.json` is read on Windows
- **THEN** the key is read and the file's permissions are left as the user's
  profile sets them

#### Scenario: A key added while Dango runs is picked up

- **WHEN** the user adds a key to `auth.json` while Dango is running and then
  runs a command using that provider
- **THEN** the request is made with the new key, without a restart

