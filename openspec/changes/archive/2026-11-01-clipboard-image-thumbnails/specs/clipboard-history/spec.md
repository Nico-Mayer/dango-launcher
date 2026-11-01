## ADDED Requirements

### Requirement: Browsing image entries stays responsive

Clipboard History SHALL respond to key presses without visible lag however
many of its listed entries are images, including large screenshots. Leaving
the history SHALL give back the memory its listed images used.

#### Scenario: Moving through many image entries

- **WHEN** the history lists as many image entries as it can show, and the
  user presses Arrow Down several times quickly
- **THEN** the selection follows each press without visible lag

#### Scenario: Typing over many image entries

- **WHEN** the history lists many image entries and the user types to narrow it
- **THEN** each character appears and the list narrows without visible lag

#### Scenario: Thumbnails stay sharp on high-density displays

- **WHEN** an image entry is listed on a display scaled above 100 %
- **THEN** its thumbnail is not visibly blurred or blocky

#### Scenario: Memory is given back after leaving the history

- **WHEN** the user leaves the history after browsing many image entries
- **THEN** Dango's memory use returns to about what it was before the history
  was opened
