## MODIFIED Requirements

### Requirement: What is observed is bounded and forgotten

The component observing keystrokes SHALL retain no more characters than the
longest keyword plus the one character before it, which the whole-word check
needs. It SHALL keep them in memory only, and SHALL never write them anywhere.

#### Scenario: Nothing typed is ever stored

- **WHEN** the user types anything at all
- **THEN** no record of it exists in the database, in a file, or in a log

#### Scenario: What is held is bounded

- **WHEN** the user types a long passage
- **THEN** no more than the longest keyword's worth of recent characters plus one is held at any moment

#### Scenario: A key that is not a character clears what is held

- **WHEN** the user presses Enter, Tab, Escape, an arrow key, or a chord with a modifier other than Shift, except the AltGr chords described below
- **THEN** what was held is discarded

#### Scenario: An AltGr character on Windows is text

- **WHEN** on Windows the user types a character that the keyboard layout makes with AltGr, such as `@`, `{`, `\` or the euro sign on a German keyboard
- **THEN** that character is held like any other typed character
- **AND** a keyword containing it expands

#### Scenario: AltGr with a key that makes no character on Windows

- **WHEN** on Windows the user presses AltGr with a key the layout gives no character
- **THEN** what was held is discarded

#### Scenario: An Option chord on macOS clears what is held

- **WHEN** on macOS the user presses a key with Option held, including a key that types a character such as `@` on a German keyboard
- **THEN** what was held is discarded
- **AND** a keyword containing such a character does not expand, a known gap left for a change made and checked on a Mac

#### Scenario: Switching application clears what is held

- **WHEN** the frontmost application changes
- **THEN** what was held is discarded

#### Scenario: A pause clears what is held

- **WHEN** the user stops typing for several seconds
- **THEN** what was held is discarded

### Requirement: Expansion is skipped where it should not happen

Dango SHALL hold nothing and expand nothing in contexts the user has excluded,
the system marks as sensitive, or that belong to Dango itself. The excluded
applications are the clipboard extension's `excluded-applications` preference,
at `extensions.dango.clipboard.preferences.excluded-applications`: one list
serves both clipboard history and keyword expansion.

On Windows, a password field is recognised only when it is a classic Win32
edit control marked as a password field. A password field in a browser or in
another custom control is not recognised, so a keyword typed there SHALL be
expected to expand.

#### Scenario: A password field on macOS

- **WHEN** the system reports that secure input is active
- **THEN** nothing is observed, nothing is held, and nothing expands

#### Scenario: A password field on Windows

- **WHEN** the user types into a classic Win32 edit control marked as a password field
- **THEN** nothing is held and nothing expands

#### Scenario: A browser password field on Windows

- **WHEN** the user types a keyword into a password field in a browser or another custom control
- **THEN** the keyword expands, because Windows offers no reliable way to know it is a password field

#### Scenario: An excluded application

- **WHEN** the user types in an application listed in the clipboard extension's `excluded-applications` preference
- **THEN** nothing is held and nothing expands

#### Scenario: The exclusion list is honoured without a restart

- **WHEN** the user changes the `excluded-applications` preference
- **THEN** the change takes effect on the next keystroke observed

#### Scenario: Typing in the launcher on Windows

- **WHEN** on Windows the user types a snippet's keyword into the launcher's search field
- **THEN** nothing expands and nothing is held
- **AND** the search field shows exactly what the user typed

#### Scenario: Typing in the launcher on macOS

- **WHEN** on macOS the user types a snippet's keyword into the launcher's search field, while the previous app is still the frontmost application
- **THEN** nothing expands and nothing is held
- **AND** the search field shows exactly what the user typed

### Requirement: Observing never delays the machine

Observation SHALL NOT make typing perceptibly slower in any application, and
SHALL NOT interfere with the keystrokes it observes. When the operating system
stops Dango's observation, Dango SHALL restore it without a restart, on Windows
and on macOS.

#### Scenario: Typing stays responsive

- **WHEN** the monitor is running and the user types continuously
- **THEN** no keystroke is perceptibly delayed by Dango's observation

#### Scenario: Keystrokes are never swallowed or altered

- **WHEN** any key is pressed, whether or not it completes a keyword
- **THEN** the application receives exactly the keystroke the user made

#### Scenario: The monitor recovers if the system stops it

- **WHEN** on macOS the operating system disables Dango's observation
- **THEN** Dango restores it
- **AND** expansion works again without a restart

#### Scenario: The monitor recovers on Windows

- **WHEN** on Windows the operating system stops delivering keystrokes to Dango's observation
- **THEN** Dango restores it
- **AND** expansion works again without a restart
