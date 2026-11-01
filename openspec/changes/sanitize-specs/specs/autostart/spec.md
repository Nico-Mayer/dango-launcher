## MODIFIED Requirements

### Requirement: Autostart is a config setting

Dango SHALL read a boolean `launcher.autostart` from the config file. When it is
true, Dango SHALL be registered to start at the user's login on the machine the
file is read on. When it is false or absent, Dango SHALL NOT be registered, and
an entry Dango created earlier SHALL be removed. The setting SHALL apply at
startup and when the config file is reloaded, without a restart. When the
config file does not parse at startup, Dango SHALL NOT apply the setting and
SHALL leave the login entry as it is.

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

#### Scenario: Config does not parse at startup

- **WHEN** Dango starts and the config file does not parse
- **THEN** the setting is not applied and the login entry is left as it is
