## ADDED Requirements

### Requirement: Matching folds case and accents and prefers word starts

Fuzzy matching of the query against a title, keyword or alias SHALL ignore
letter case and SHALL treat an unaccented Latin letter in the query as
matching its accented forms in the text. Among texts that contain the query
as a subsequence, a match that lands on the start of the text or on word
starts SHALL score higher than one that lands mid-word, and a contiguous match
SHALL score higher than a scattered one. The positions reported for
highlighting SHALL index characters, not bytes, so an accented title
highlights correctly.

#### Scenario: Case is ignored

- **WHEN** the user types `FIRE` and an item is titled "Firefox"
- **THEN** the item matches

#### Scenario: Accents are folded

- **WHEN** the user types `cafe` and an item is titled "Café"
- **THEN** the item matches
- **AND** the highlighted positions mark the four characters of the title

#### Scenario: A word start beats a mid-word hit

- **WHEN** the user types `ze`, and "Zed" and "Maximize" have equal frecency and no learned input
- **THEN** "Zed" ranks above "Maximize"

#### Scenario: A contiguous match beats a scattered one

- **WHEN** the user types `fire`, and "Firefox" and "Far Infrared Emitter" have equal frecency and no learned input
- **THEN** "Firefox" ranks above "Far Infrared Emitter"
