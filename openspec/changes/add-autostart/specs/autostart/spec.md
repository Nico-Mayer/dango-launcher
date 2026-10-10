## Purpose

Starting Dango automatically when the user logs in, controlled from the config
file like every other setting, so the launcher is there on both machines
without being started by hand.

## ADDED Requirements

### Requirement: Autostart is a config setting

Dango SHALL read a boolean `launcher.autostart` from the config file. When it is
true, Dango SHALL be registered to start at the user's login on the machine the
file is read on. When it is false or absent, Dango SHALL NOT be registered, and
an entry Dango created earlier SHALL be removed. The setting SHALL apply at
startup and when the config file is reloaded, without a restart.

#### Scenario: Setting is absent

- **WHEN** the config file does not mention `launcher.autostart`, or there is no config file
- **THEN** Dango is not registered to start at login

#### Scenario: Setting is turned on in the file

- **WHEN** the user sets `launcher.autostart` to true and saves the file while Dango is running
- **THEN** Dango is registered to start at login before the next login, with no restart

#### Scenario: Setting is turned off in the file

- **WHEN** the user sets `launcher.autostart` to false, or removes it, and saves the file while Dango is running
- **THEN** the login entry Dango created is removed before the next login

#### Scenario: Setting is on at startup

- **WHEN** Dango starts and the file says `launcher.autostart` is true
- **THEN** the login entry exists afterwards and points at the application that just started, even if it previously pointed somewhere else

#### Scenario: Setting is unchanged on reload

- **WHEN** the config file is reloaded and the setting has the same value as before
- **THEN** the login entry is left as it is and no error is reported

### Requirement: The login entry starts Dango the way the user would

The login entry SHALL start the installed application so that it behaves
exactly as a manual start: tray icon present, launcher window hidden, hotkey
registered, no window or focus taken from whatever the user is doing. The entry
SHALL belong to the logged-in user only and SHALL NOT require elevation to
create or remove.

#### Scenario: Login on Windows

- **WHEN** the setting is on and the user logs in to Windows
- **THEN** Dango is running within the normal startup-app window, with its tray icon shown and no visible launcher window
- **AND** the entry is listed under the user's startup apps, not the machine's

#### Scenario: Login on macOS

- **WHEN** the setting is on and the user logs in to macOS
- **THEN** Dango is running with its menu bar icon shown and no visible launcher window
- **AND** the entry is a per-user launch agent that the system lists among the items allowed to run in the background

#### Scenario: The config file wins over the Windows startup toggle

- **WHEN** the user disables Dango in the Windows startup-apps settings while the config setting is on
- **THEN** Dango does not start at the next login
- **AND** the next time Dango runs with the setting still on, the entry is enabled again, because the file is where the setting is turned off

### Requirement: A development build never registers itself

A debug build SHALL NOT create, update, or remove a login entry, whatever the
config file says, so a development run never leaves a login entry that points
at a build directory.

#### Scenario: Debug run with the setting on

- **WHEN** a debug build starts with `launcher.autostart` set to true
- **THEN** no login entry is created or changed
- **AND** `dango.log` says the setting was ignored because this is a development build

### Requirement: A failure to apply the setting is visible

If the login entry cannot be created or removed, Dango SHALL keep running, SHALL
write the reason to `dango.log`, and SHALL show a disabled tray item that says
autostart could not be applied and names the log. The item SHALL be present at
startup when the failure happened at startup; a failure during a reload SHALL be
reported through the config status line and the log.

#### Scenario: Registration fails at startup

- **WHEN** the setting is on and the entry cannot be written at startup
- **THEN** Dango runs normally and the tray shows a disabled item naming `dango.log`
- **AND** the log says what could not be written

#### Scenario: Registration fails on reload

- **WHEN** the user turns the setting on and the entry cannot be written
- **THEN** the config status line reports an error, the log says why, and Dango keeps the rest of the reloaded configuration
