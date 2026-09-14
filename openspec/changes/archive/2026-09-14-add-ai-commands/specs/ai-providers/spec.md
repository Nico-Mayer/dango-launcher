## Purpose

Where the answers come from: which model services Dango can talk to, how the
user names one and points it at a machine or a hosted endpoint, where its key is
kept, and what the user is told when a request cannot be made or fails.

## ADDED Requirements

### Requirement: A provider is named in configuration

A provider SHALL be an entry the user names in the configuration file, carrying
the kind of service it speaks to, an endpoint where one applies, and a default
model. Four kinds SHALL be supported: Anthropic's own protocol, an
OpenAI-compatible endpoint, Ollama on the local machine, and a command on the
local machine. No provider SHALL be assumed: with none configured, the AI
commands SHALL say so rather than failing when they are run.

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

#### Scenario: No provider is configured

- **WHEN** the user runs an AI command and no provider is configured
- **THEN** nothing is sent
- **AND** the launcher says that no AI provider is set up and where to add one

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
- **THEN** the command fails with a message saying the local model could not be
  reached
- **AND** the message says to check that the local model server is running

### Requirement: A provider may be a command on this machine

A provider MAY be a program to run rather than a service to call: one that takes
a prompt and prints an answer. Such a provider SHALL name the command and the
arguments to pass it. The prompt SHALL reach the command on its standard input,
unless the arguments say where it goes.

A command provider SHALL need no credential, and SHALL NOT be looked up in the
key file. Its answer SHALL stream as the command prints it, and SHALL be subject
to the same abandoning, output settings, and result actions as any other answer.

Dango SHALL run the command in an empty working directory, so a program that can
touch files has nothing of the user's within reach.

#### Scenario: A command answers a transform

- **WHEN** a provider names a command that prints an answer, and a command using
  it is run
- **THEN** the prompt reaches the program
- **AND** what it prints appears as the answer

#### Scenario: The prompt goes where the arguments say

- **WHEN** a provider's arguments contain a placeholder for the prompt
- **THEN** the prompt is passed as that argument rather than on standard input

#### Scenario: The model is named the way the program expects

- **WHEN** a provider's arguments contain a placeholder for the model
- **THEN** the model the command resolved to is put in its place

#### Scenario: A command provider needs no key

- **WHEN** a command provider is used and `auth.json` has no entry for it
- **THEN** the request is made anyway
- **AND** no key is read for it

#### Scenario: The program is not installed

- **WHEN** the named command cannot be run
- **THEN** the failure says which command could not be run and to check that it
  is installed

#### Scenario: The program fails

- **WHEN** the command exits without printing an answer
- **THEN** the failure says the provider gave no answer and where to look
- **AND** what the program printed on its error output goes to the log

#### Scenario: The answer arrives while the program is still running

- **WHEN** a command prints its answer over several seconds
- **THEN** the text appears as it is printed rather than only at the end

#### Scenario: Abandoning stops the program

- **WHEN** the user abandons an answer a command provider is still producing
- **THEN** the program is stopped rather than left running

#### Scenario: The program cannot see the user's files

- **WHEN** a command provider runs
- **THEN** its working directory is an empty one, not the user's own

### Requirement: Keys live in a plain file in the config directory

Keys for hosted providers SHALL be read from `auth.json` in the configuration
directory, `~/.config/dango/` on both platforms, honouring `$DANGO_CONFIG_DIR`.
The file SHALL map a provider's name to its key. It SHALL be optional: with no
file, only providers needing no key work.

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

- **WHEN** `auth.json` exists but does not parse
- **THEN** Dango keeps running and surfaces the problem the way a bad
  configuration file is surfaced
- **AND** commands using a local provider still work

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
model SHALL be used when no model is named anywhere.

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

#### Scenario: Several providers and no default

- **WHEN** more than one provider is configured, no default is named, and a
  command names none either
- **THEN** it fails with a message saying no default is set and where to set one
- **AND** no request is made

#### Scenario: A command names a provider that is not configured

- **WHEN** a command names a provider with no entry in the configuration
- **THEN** it fails with a message saying that provider is not set up
- **AND** no request is made

### Requirement: A request failure is explained in the user's terms

A request that is refused, times out, or cannot be reached SHALL leave the user a
message saying what did not happen and what to do about it. The message SHALL
NOT be a raw error from the service or the HTTP layer, and detail useful only
for debugging SHALL go to the log.

#### Scenario: The key is rejected

- **WHEN** the service refuses the request because the key is not valid
- **THEN** the message says the provider did not accept the key and to check it
  in the key file

#### Scenario: The model does not exist

- **WHEN** the service reports that the named model is unknown
- **THEN** the message says that model is not available from that provider

#### Scenario: The service is unreachable

- **WHEN** the request cannot reach the service
- **THEN** the message says the provider could not be reached and to check the
  connection

#### Scenario: The service is rate limiting

- **WHEN** the service refuses because too many requests have been made
- **THEN** the message says the provider is busy and to try again shortly

#### Scenario: Nothing arrives in time

- **WHEN** no part of an answer has arrived within 30 seconds of the request
- **THEN** the request is abandoned
- **AND** the message says the model did not answer in time

### Requirement: Requests are made from the application, never the webview

Every request to a provider SHALL be made by the application itself. No key and
no request SHALL be handed to the webview, and no provider endpoint SHALL be
called from it.

#### Scenario: The webview never holds a key

- **WHEN** an AI command runs
- **THEN** the key and the request stay inside the application
- **AND** the webview receives only the text of the answer
