## MODIFIED Requirements

### Requirement: A local provider needs no credential

A provider running on the user's own machine SHALL work with no key. Dango SHALL
NOT ask for, require, or send a credential for one, and the text of a request to
a local provider SHALL NOT leave the machine.

#### Scenario: A local model answers without a key

- **WHEN** an Ollama provider is configured and no key exists for it
- **THEN** a command using that provider runs normally

#### Scenario: A local endpoint is not running

- **WHEN** a command uses a local provider and nothing is listening at its
  endpoint
- **THEN** the command fails with "Couldn't reach your local model server"

### Requirement: Keys live in a plain file in the config directory

Keys for hosted providers SHALL be read from `auth.json` in the configuration
directory described in `openspec/specs/configuration`. The file SHALL map a
provider's name to its key. It SHALL be optional: with no file, only providers
needing no key work.

Dango SHALL only read this file; it SHALL NOT write keys into it. A key SHALL
NOT be written into `config.json`, into the local database, into a log, or into
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

### Requirement: A command chooses its own provider, model, and thinking level

Each command SHALL be able to name the provider it uses, the model, and how much
the model should think before answering. Anything a command does not name SHALL
fall back to the configured default, and the default provider's own default
model SHALL be used when no model is named anywhere. With exactly one provider
configured and no default named, that provider SHALL be the default.

The thinking level SHALL be one of off, low, medium, or high. A model that
cannot think SHALL ignore the level rather than failing.

#### Scenario: Two commands use different models

- **WHEN** one command names a local model and another names a hosted one, and
  both are run
- **THEN** each request goes to the service that command named

#### Scenario: A command names only a model

- **WHEN** a command names a model but no provider
- **THEN** the default provider is used with that model

#### Scenario: A command names nothing

- **WHEN** a command names neither provider nor model
- **THEN** the default provider and its default model are used

#### Scenario: A thinking level is requested

- **WHEN** a command sets its thinking level to high and the model supports
  thinking
- **THEN** the request asks for that level of thinking

#### Scenario: A model that cannot think

- **WHEN** a command sets a thinking level and the model does not support one
- **THEN** the request is made anyway and the answer arrives normally

#### Scenario: One provider and no default

- **WHEN** exactly one provider is configured, no default is named, and a
  command names none either
- **THEN** that provider is used

#### Scenario: Several providers and no default

- **WHEN** more than one provider is configured, no default is named, and a
  command names none either
- **THEN** it fails with a message saying no default is set and where to set one
- **AND** no request is made

#### Scenario: A command names a provider that is not configured

- **WHEN** a command names a provider with no entry in the configuration
- **THEN** it fails with a message saying that provider is not set up
- **AND** no request is made

#### Scenario: No model is named anywhere

- **WHEN** neither the command nor its provider names a model
- **THEN** it fails with "Set a model for {provider} in your config file", with
  the provider's name in place of `{provider}`
- **AND** no request is made

## REMOVED Requirements

### Requirement: Requests are made by the application core, never the UI layer

**Reason**: Written for the WebView, where the interface was a separate
process. With Slint the interface is drawn in the same process and only
receives the snapshot, so the rule no longer separates anything observable.

**Migration**: That a key never appears in a message shown on screen is
covered by "Keys live in a plain file in the config directory".
