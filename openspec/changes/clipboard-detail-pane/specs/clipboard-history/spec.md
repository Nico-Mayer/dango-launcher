## ADDED Requirements

### Requirement: The selected entry is shown in full

The history SHALL show the selected entry beside the list. A text entry SHALL
be shown as it was copied, with its line breaks and indentation, and never
read as formatting. A long text SHALL show at least its start. An image entry
SHALL be shown whole, scaled to fit the pane.

#### Scenario: A text entry is shown as copied

- **WHEN** the user selects a text entry that spans several indented lines
- **THEN** the pane shows the text with its line breaks and indentation

#### Scenario: Markup characters stay as typed

- **WHEN** the user selects a text entry containing `# heading` and `*stars*`
- **THEN** the pane shows those characters as typed, not as a heading or emphasis

#### Scenario: An image entry is shown whole

- **WHEN** the user selects an image entry
- **THEN** the pane shows the whole image, scaled to fit, with its aspect ratio kept

#### Scenario: Moving the selection changes the pane

- **WHEN** the user presses Arrow Down in the history
- **THEN** the pane shows the newly selected entry

### Requirement: An entry shows where and when it was copied

Below the entry, the pane SHALL show metadata in this order, leaving out a pair
that does not apply:

- "Source": the application the entry was copied from, when it is known.
- "Copied": when it was last copied, as "Today, 14:31", "Yesterday, 14:31",
  "27 Oct, 14:31" within the current year, or "27 Oct 2025, 14:31" before it,
  in the user's local time.
- "Type": "Text", "Link", "Email address", "Colour", or "File path" for text,
  and "Image" for an image.
- "Lines": the number of lines, for text with more than one line.
- "Characters": the number of characters in the whole text, for text.
- "Size": the width and height in pixels, as "1920 × 1080", for an image.

Copying something already in the history SHALL update its source and its time.

#### Scenario: Source on Windows

- **WHEN** the user copies text in an application on Windows and selects that
  entry in the history
- **THEN** the pane shows "Source" with that application's name

#### Scenario: Source on macOS with one candidate

- **WHEN** the user copies text on macOS while only one application was
  frontmost around the copy, and selects that entry
- **THEN** the pane shows "Source" with that application's name

#### Scenario: Source on macOS with several candidates

- **WHEN** the user copies text on macOS and switches applications before the
  copy is noticed, so several applications could have made it
- **THEN** the pane shows no "Source" pair for that entry

#### Scenario: Source unknown

- **WHEN** an entry was copied by a program that leaves no owner, such as a
  command-line tool
- **THEN** the pane shows no "Source" pair

#### Scenario: Copied time today

- **WHEN** the user selects an entry copied today at 14:31 local time
- **THEN** the pane shows "Copied" with "Today, 14:31"

#### Scenario: Copied time in an earlier year

- **WHEN** the user selects an entry copied on 27 October of last year at 14:31
- **THEN** the pane shows "Copied" with the date including the year and the time

#### Scenario: A link is recognised

- **WHEN** the user selects a text entry that is a single web address
- **THEN** the pane shows "Type" with "Link"

#### Scenario: A colour is recognised

- **WHEN** the user selects a text entry that is a hex colour such as `#ff8800`
- **THEN** the pane shows "Type" with "Colour"

#### Scenario: Plain text

- **WHEN** the user selects a text entry that is none of the recognised kinds
- **THEN** the pane shows "Type" with "Text"

#### Scenario: Text size

- **WHEN** the user selects a text entry of three lines and 38 characters
- **THEN** the pane shows "Lines" with "3" and "Characters" with "38"

#### Scenario: Long text counts in full

- **WHEN** the user selects a text entry longer than the pane shows
- **THEN** "Characters" counts the whole text

#### Scenario: Image size

- **WHEN** the user selects an image entry 1920 pixels wide and 1080 high
- **THEN** the pane shows "Size" with "1920 × 1080"

#### Scenario: Copying again updates the metadata

- **WHEN** the user copies text in one application, then copies the same text
  in another application
- **THEN** the entry shows the second application as its source and the second
  copy's time

## MODIFIED Requirements

### Requirement: Browsing image entries stays responsive

Clipboard History SHALL respond to key presses without visible lag however
many of its listed entries are images, including large screenshots, and with
the selected image shown in the pane. Leaving the history SHALL give back the
memory its listed and shown images used.

#### Scenario: Moving through many image entries

- **WHEN** the history lists as many image entries as it can show, and the
  user presses Arrow Down several times quickly
- **THEN** the selection follows each press without visible lag
- **AND** the pane shows each newly selected image without visible lag

#### Scenario: Moving through large screenshots

- **WHEN** the history lists several full-screen screenshots and the user moves
  the selection between them
- **THEN** the selection and the pane follow without visible lag

#### Scenario: Typing over many image entries

- **WHEN** the history lists many image entries and the user types to narrow it
- **THEN** each character appears and the list narrows without visible lag

#### Scenario: Thumbnails stay sharp on high-density displays

- **WHEN** an image entry is listed on a display scaled above 100 %
- **THEN** its thumbnail is not visibly blurred or blocky

#### Scenario: The pane image stays sharp on high-density displays

- **WHEN** an image entry larger than the pane is selected on a display scaled
  above 100 %
- **THEN** the image in the pane is not visibly blurred or blocky

#### Scenario: Memory is given back after leaving the history

- **WHEN** the user leaves the history after browsing many image entries
- **THEN** Dango's memory use returns to about what it was before the history
  was opened
