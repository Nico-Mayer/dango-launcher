## MODIFIED Requirements

### Requirement: A provider is named in configuration

A provider SHALL be an entry the user names in the configuration file, carrying
the kind of service it speaks to, an endpoint where one applies, and a default
model. Three kinds SHALL be supported: Anthropic's own protocol, an
OpenAI-compatible endpoint, and Ollama on the local machine. No provider SHALL
be assumed: with none configured, the AI commands SHALL say so rather than
failing when they are run.

#### Scenario: A hosted provider is configured

- **WHEN** the user adds a provider of the Anthropic kind with a default model
- **THEN** commands can name that provider and reach that service

#### Scenario: An OpenAI-compatible endpoint is configured

- **WHEN** the user adds a provider naming an OpenAI-compatible base URL, such
  as a hosted router or a local server
- **THEN** commands naming that provider send their requests to that endpoint

#### Scenario: An unknown provider kind

- **WHEN** a provider entry names a kind Dango does not support
- **THEN** that provider is ignored and the problem is surfaced the way a
  configuration problem is
- **AND** the other providers still work

#### Scenario: A program is not a provider kind

- **WHEN** a provider entry names the kind `cli`
- **THEN** it is treated as an unknown provider kind

#### Scenario: No provider is configured

- **WHEN** the user runs an AI command and no provider is configured
- **THEN** nothing is sent
- **AND** the launcher says that no AI provider is set up and where to add one

## REMOVED Requirements

### Requirement: A provider may be a command on this machine

**Reason**: Running an agent CLI per request proved fragile on Windows (a
console window stole focus and cancelled the run), needed its own PATH setting,
and loaded the user's whole agent setup for every request. Making it reliable
needs per-vendor knowledge Dango should not carry.

**Migration**: Point the AI commands at an Ollama, Anthropic, or
OpenAI-compatible provider. Remove `"kind": "cli"` entries and the
`dango.ai.path` setting from the config file.
