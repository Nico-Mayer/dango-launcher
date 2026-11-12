# Spec Delta

## MODIFIED Requirements

### Requirement: Discovery on macOS

On macOS the extension SHALL discover application bundles in the system
applications directory, the user's applications directory, the system's own
applications directory, the system's core services applications directory, and
their immediate subdirectories. It SHALL also discover Finder. It SHALL NOT
discover the other bundles in the core services directory, which are background
agents and helpers rather than applications a user opens.

#### Scenario: User-installed application is found

- **WHEN** an application bundle is present in the user's applications directory
- **THEN** it appears in the index

#### Scenario: Application in a subfolder is found

- **WHEN** an application bundle sits inside a category subfolder of an applications directory
- **THEN** it appears in the index

#### Scenario: Finder is found

- **WHEN** the user types "Finder"
- **THEN** Finder appears in the results
- **AND** choosing it brings Finder to the foreground

#### Scenario: Core services application is found

- **WHEN** the user types the name of an application in the core services applications directory, such as Keychain Access or Archive Utility
- **THEN** that application appears in the results

#### Scenario: Background agents stay out

- **WHEN** the user types the name of a background agent in the core services directory, such as Dock or WiFiAgent
- **THEN** no result for that agent appears

#### Scenario: Display name honours localisation

- **WHEN** an application's bundle carries a localised display name
- **THEN** that localised name is used

#### Scenario: Display name follows the system

- **WHEN** an application declares a display name that is not a localisation of
  its bundle filename, which macOS itself disregards
- **THEN** the name shown matches the one the system shows, so a launcher result
  never disagrees with Finder
