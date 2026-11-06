# ai-providers Specification

## Purpose

Where the answers come from: which model services Dango can talk to, how the
user names one and points it at a machine or a hosted endpoint, where its key is
kept, and what the user is told when a request cannot be made or fails.

## Requirements

### Requirement: A provider is named in configuration

A provider SHALL be an entry the user names in the configuration file, carrying
the kind of service it speaks to, an endpoint where one applies, and a default
model. Five kinds SHALL be supported: Anthropic's own protocol, an
OpenAI-compatible endpoint, Ollama on the local machine, and the agent programs
Claude Code and pi. No provider SHALL be assumed: with none configured, the AI
commands SHALL say so rather than failing when they are run.

#### Scenario: A hosted provider is configured

- **WHEN** the user adds a provider of the Anthropic kind with a default model
- **THEN** commands can name that provider and reach that service

#### Scenario: An OpenAI-compatible endpoint is configured

- **WHEN** the user adds a provider naming an OpenAI-compatible base URL, such
  as a hosted router or a local server
- **THEN** commands naming that provider send their requests to that endpoint

#### Scenario: An agent program is configured

- **WHEN** the user adds a provider of the `claude` or `pi` kind
- **THEN** commands naming that provider are answered by that program

#### Scenario: An unknown provider kind

- **WHEN** a provider entry names a kind Dango does not support
- **THEN** the problem is surfaced the way a configuration problem is
- **AND** the message names that kind

#### Scenario: A program is not a provider kind

- **WHEN** a provider entry names the kind `cli` or `codex`
- **THEN** it is treated as an unknown provider kind

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
- **THEN** the command fails with "Couldn't reach your local model server"

### Requirement: Keys live in a plain file in the config directory

Keys for hosted providers SHALL be read from `auth.toml`, a user-owned file
under the contract in `config-files`. The file SHALL map a provider's name to
its key, as in `anthropic = "sk-ant-..."`. It SHALL be optional: with no
file, only providers needing no key work. The `extensions` table SHALL hold
secrets of JavaScript extensions, as `js-extensions` describes, and SHALL NOT
be read as a provider's key.

Dango SHALL only read this file; it SHALL NOT write keys into it. A key SHALL
NOT be written into `config.toml`, into the local database, into a log, or into
any message shown on screen. A parse error in the file SHALL name the line
and SHALL NOT include any text from the file. Where the platform expresses file permissions,
Dango SHALL keep the file readable only by its owner.

#### Scenario: A key is read for a hosted provider

- **WHEN** `auth.toml` holds a key for a provider and a command uses that
  provider
- **THEN** the request is made with that key

#### Scenario: The key file is missing

- **WHEN** a command uses a hosted provider and no `auth.toml` exists
- **THEN** the command fails with a message naming the provider and saying a key
  is needed
- **AND** the message names the file to put it in

#### Scenario: The key file cannot be read

- **WHEN** `auth.toml` exists but does not parse and a command uses a hosted
  provider
- **THEN** the problem is written to the log
- **AND** the command fails with "auth.toml couldn't be read. Check the file in
  your config folder."

#### Scenario: A broken key file shows no key

- **WHEN** `auth.toml` holds `anthropic = "sk-ant-secret` with no closing
  quote and a command uses a hosted provider
- **THEN** the log names `auth.toml` and the line
- **AND** `sk-ant-secret` appears in neither the log nor the message

#### Scenario: A local provider works with an unreadable key file

- **WHEN** `auth.toml` exists but does not parse and a command uses a local
  provider
- **THEN** the command runs normally

#### Scenario: A key never appears anywhere else

- **WHEN** a request has been made with a key and the log, the configuration
  file, and every message shown are inspected
- **THEN** the key appears in none of them

#### Scenario: The key file is restricted to its owner on macOS

- **WHEN** `auth.toml` is readable by users other than its owner and Dango reads
  it
- **THEN** Dango restricts it to its owner
- **AND** the key is still read

#### Scenario: The key file on Windows

- **WHEN** `auth.toml` is read on Windows
- **THEN** the key is read and the file's permissions are left as the user's
  profile sets them

#### Scenario: A key added while Dango runs is picked up

- **WHEN** the user adds a key to `auth.toml` while Dango is running and then
  runs a command using that provider
- **THEN** the request is made with the new key, without a restart

#### Scenario: The extensions table is not a provider

- **WHEN** `auth.toml` holds an `[extensions.gitlab]` table and an AI command
  names a provider called `extensions`
- **THEN** no key is read from that table for the provider

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

- **WHEN** neither the command nor its provider names a model, and the provider
  is not an agent program
- **THEN** it fails with "Set a model for {provider} in your config file", with
  the provider's name in place of `{provider}`
- **AND** no request is made

#### Scenario: An agent program with no model named

- **WHEN** neither the command nor its provider names a model, and the provider
  is an agent program
- **THEN** the program is run with whatever model it uses by default

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

- **WHEN** the provider is a service and no part of an answer has arrived
  within 30 seconds of the request
- **THEN** the request is abandoned
- **AND** the message says the model did not answer in time

#### Scenario: An agent program goes quiet

- **WHEN** the provider is an agent program and two minutes pass since it
  started or since it last printed something, without new output
- **THEN** the program is stopped
- **AND** the message says the model did not answer in time

#### Scenario: An agent program that keeps printing is not cut off

- **WHEN** an agent program runs longer than two minutes but never goes two
  minutes without new output
- **THEN** it is allowed to finish
- **AND** its whole answer is shown

### Requirement: An agent program answers with what it prints

A provider of the `claude` or `pi` kind SHALL run that program once per request,
give it the prompt, and use the text it prints as the answer, shown as it
arrives. Dango SHALL NOT look up or send a key for these kinds: the program uses
the sign-in the user set up for it.

#### Scenario: A command is answered by an agent program

- **WHEN** a command uses a `claude` provider and the program is installed and
  signed in
- **THEN** the launcher shows that it is working until output arrives
- **AND** then shows the program's printed answer

#### Scenario: No key is needed

- **WHEN** a command uses an agent program provider and `auth.toml` holds no
  key for it, or does not exist
- **THEN** the command runs normally

#### Scenario: The named model reaches the program

- **WHEN** a command names a model and uses an agent program provider
- **THEN** the program is asked to use that model

#### Scenario: The thinking level reaches the program

- **WHEN** a command sets a thinking level other than off and uses an agent
  program provider
- **THEN** the program is asked for that level of effort

#### Scenario: The program's chatter is not the answer

- **WHEN** the program writes progress or warnings on its error output
- **THEN** that text goes to the log
- **AND** it does not appear in the answer

### Requirement: An agent program runs without the user's agent setup

An agent program run for a command SHALL NOT use tools, MCP servers, hooks,
plugins, skills, extensions, or context files such as CLAUDE.md and AGENTS.md,
and SHALL NOT save the run as a session the user could resume. It SHALL run in
an empty folder of its own.

#### Scenario: Tools are off

- **WHEN** a prompt asks the program to read a file or run a command
- **THEN** the program has no tool to do it with
- **AND** nothing on the user's disk is read or changed

#### Scenario: The user's hooks and MCP servers are not loaded

- **WHEN** the user has hooks or MCP servers set up for the program and a
  command uses it
- **THEN** no hook runs and no MCP server is started for that run

#### Scenario: The user's context files are not loaded

- **WHEN** the user has a global CLAUDE.md or AGENTS.md with instructions for
  the program and a command uses it
- **THEN** those instructions do not reach the model

#### Scenario: No session is left behind

- **WHEN** a command has been answered by an agent program
- **THEN** that run does not appear among the program's sessions to resume

### Requirement: Dango finds the agent program

When a command uses an agent program provider, Dango SHALL find the program at
that moment. A configured absolute path SHALL be used as is. Without one, Dango
SHALL look on its own PATH, the PATH the user's login shell sets up, and the
usual install folders of version managers and package managers. A program
installed into one of those places while Dango runs SHALL be found without a
restart.

#### Scenario: A configured path is used

- **WHEN** a provider entry names an absolute path to the program
- **THEN** that file is run, wherever it is

#### Scenario: A program installed through a version manager on Windows

- **WHEN** the program is reachable only through mise's shims or as an npm
  wrapper script, and Dango was started at login
- **THEN** Dango finds and runs it

#### Scenario: A program on the shell's PATH on macOS

- **WHEN** the program's folder is added to PATH only in the user's shell
  profile, and Dango was opened from Finder or at login
- **THEN** Dango finds and runs it

#### Scenario: The program is not installed

- **WHEN** no path is configured and the program cannot be found
- **THEN** nothing is run
- **AND** the command fails with "Couldn't find {program}. Install it or set its
  path in your config file.", with the program's name in place of `{program}`

#### Scenario: The configured path is wrong

- **WHEN** a provider entry names a path that is not absolute or does not exist
- **THEN** nothing is run
- **AND** the command fails with "Couldn't find {path}. Check the path in your
  config file.", with the configured path in place of `{path}`

#### Scenario: The program is installed while Dango runs

- **WHEN** the command failed because the program was missing, the user then
  installs it into one of the searched places, and runs the command again
- **THEN** the program is found and run, without a restart

#### Scenario: A slow login shell does not block the launcher

- **WHEN** the user's login shell takes long to start or waits for input while
  Dango looks for the program
- **THEN** the launcher stays responsive and Escape abandons the request
- **AND** the search goes on without the login shell's PATH

### Requirement: An agent program runs out of sight

Running an agent program SHALL NOT show a window or take focus away from the
launcher, and stopping a request SHALL stop the program and every process it
started.

#### Scenario: No console window on Windows

- **WHEN** a command uses an agent program in a release build on Windows
- **THEN** no console or other window appears
- **AND** the launcher stays the foreground window and keeps its view

#### Scenario: No window on macOS

- **WHEN** a command uses an agent program on macOS
- **THEN** no window or Dock icon appears
- **AND** the launcher keeps focus

#### Scenario: Abandoning stops the program

- **WHEN** an agent program is running for a command and the request is
  abandoned, by Escape, by a reset, or by another command
- **THEN** the program and every process it started are stopped
- **AND** the launcher handles the next key without delay

#### Scenario: Quitting Dango stops the program

- **WHEN** an agent program is running for a command and the user quits Dango
  from the tray
- **THEN** the program and every process it started are stopped

#### Scenario: A process left behind by the program is stopped

- **WHEN** the program has finished printing but a process it started is still
  running, and the request is abandoned
- **THEN** that process is stopped too

### Requirement: An agent program's failure is explained

When an agent program ends with an error, the command SHALL fail with a message
saying so, and what the program wrote on both its outputs SHALL go to the log.

#### Scenario: The program fails

- **WHEN** the program exits with an error, for example because the user is not
  signed in
- **THEN** the command fails with "{provider} stopped with an error. See
  dango.log for the reason.", with the provider's name in place of `{provider}`
- **AND** the program's standard output and error output are in the log

#### Scenario: The program succeeds but prints nothing

- **WHEN** the program exits without error and prints no text
- **THEN** the launcher says the model returned no text
