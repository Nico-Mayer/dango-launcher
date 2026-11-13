# screenshots Specification

## Purpose

The built-in `screenshots` extension finds the screenshots the user has saved and pastes or copies them from the keyboard, so a recent screenshot reaches a chat or editor without a trip to the file manager.

## Requirements

### Requirement: The extension contributes three commands

The `screenshots` extension, named "Screenshots", SHALL contribute `Search Screenshots`, a view command, and `Paste Latest Screenshot` and `Copy Latest Screenshot`, two no-view commands. All three SHALL be found in root search like any other command and SHALL be bindable to a hotkey.

#### Scenario: Commands are found in root search

- **WHEN** the user types "screenshot" in root search
- **THEN** `Search Screenshots`, `Paste Latest Screenshot`, and `Copy Latest Screenshot` are listed

### Requirement: Screenshots are image files in the screenshot folders

A screenshot SHALL be a file with the extension png, jpg, jpeg, or webp, matched ignoring case, directly inside one of the screenshot folders. Subfolders SHALL NOT be searched. The folders SHALL be read each time a command runs, so a screenshot saved a moment ago is included.

#### Scenario: A new screenshot is included at once

- **WHEN** the user takes a screenshot and then invokes `Search Screenshots`
- **THEN** the new screenshot is the first tile

#### Scenario: Other files are ignored

- **WHEN** a screenshot folder holds a `.mov` recording, a `.pdf`, and a subfolder of images
- **THEN** none of them is shown

#### Scenario: Upper-case extensions count

- **WHEN** a screenshot folder holds `Capture.PNG`
- **THEN** it is shown

### Requirement: The screenshot folders default to where the system saves screenshots

Without the `folders` setting, the screenshot folder SHALL be the one the operating system's own screenshot tools save to.

#### Scenario: Default folder on macOS

- **WHEN** `folders` is not set and the user has chosen a save location for screenshots in the macOS Screenshot app
- **THEN** that location is the screenshot folder

#### Scenario: Default folder on macOS without a chosen location

- **WHEN** `folders` is not set and the user has never chosen a save location on macOS
- **THEN** the Desktop is the screenshot folder

#### Scenario: Default folder on Windows

- **WHEN** `folders` is not set on Windows
- **THEN** the Screenshots folder that Win+PrtScn and Snipping Tool save to is the screenshot folder, including when it was moved or redirected to OneDrive

### Requirement: The folders setting replaces the default

The setting `folders` in `[screenshots]` SHALL be a list of folder paths. When set and not empty, it SHALL replace the default folder, and screenshots from every listed folder SHALL be shown together. A path starting with `~` SHALL be read from the user's home folder. A listed folder that does not exist SHALL be skipped without failing the command.

#### Scenario: Two folders are merged

- **WHEN** `folders = ["~/Desktop", "~/Pictures/screenshots"]` and both hold screenshots
- **THEN** `Search Screenshots` shows the screenshots of both folders in one grid, newest first

#### Scenario: A missing folder is skipped

- **WHEN** one listed folder does not exist and another holds screenshots
- **THEN** the screenshots of the existing folder are shown

#### Scenario: An empty list means the default

- **WHEN** `folders = []`
- **THEN** the default folder is used

### Requirement: Screenshots are browsed in a grid

`Search Screenshots` SHALL show the screenshots in a grid of thumbnails, newest first by the time the file was last modified, with the file name as each tile's title. Typing SHALL narrow the grid by file name. Opening the grid SHALL NOT wait for thumbnails, and thumbnails SHALL be small enough that the grid stays responsive with many large screenshots.

#### Scenario: Grid opens newest first

- **WHEN** the user invokes `Search Screenshots`
- **THEN** the most recently modified screenshot is the first tile

#### Scenario: Grid narrows by file name

- **WHEN** the user types part of a screenshot's file name
- **THEN** the grid narrows to the screenshots whose names match

#### Scenario: Large screenshots stay responsive

- **WHEN** the folder holds fifty 5K screenshots and the user opens the grid and moves the selection
- **THEN** there is no visible lag

### Requirement: The grid explains an empty result

When no screenshot is found, the grid SHALL show an empty state. When no screenshot folder exists, the empty state SHALL say so and point to the config file. Otherwise it SHALL say no screenshots were found.

#### Scenario: No screenshots yet

- **WHEN** the screenshot folders exist but hold no screenshots
- **THEN** the grid shows "No screenshots yet" with "Screenshots you save will show up here"

#### Scenario: No folder exists

- **WHEN** none of the screenshot folders exists
- **THEN** the grid shows "Screenshot folder not found" with "Check the screenshot folders in the config file"

### Requirement: A screenshot can be pasted or copied

Each tile SHALL offer "Paste", run by Enter, and "Copy" in the action panel. Paste SHALL hide the launcher and insert the screenshot into the previous app. Copy SHALL place it on the clipboard and report "Copied to clipboard". Both SHALL put the screenshot on the clipboard as an image and as a file at once, so an app that takes pasted images gets the image and a file manager gets the file.

#### Scenario: Enter pastes into a chat app

- **WHEN** the user presses Enter on a screenshot while a chat app's message field was focused
- **THEN** the launcher hides
- **AND** the screenshot is pasted into the message field as an image

#### Scenario: Copied screenshot pastes as a file in the file manager

- **WHEN** the user copies a screenshot and pastes in a Finder window on macOS
- **THEN** a copy of the screenshot file appears in that folder

#### Scenario: Copied screenshot pastes as a file in File Explorer

- **WHEN** the user copies a screenshot and pastes in a File Explorer window on Windows
- **THEN** a copy of the screenshot file appears in that folder

#### Scenario: Copied screenshot pastes as an image in an image editor

- **WHEN** the user copies a screenshot and pastes into an image editor
- **THEN** the editor receives the image

#### Scenario: A screenshot deleted meanwhile

- **WHEN** the user pastes or copies a screenshot whose file was deleted after the grid opened
- **THEN** the failure ticker reads "That screenshot no longer exists"
- **AND** the clipboard is unchanged

### Requirement: A screenshot can be opened, shown, or its path copied

Each tile's action panel SHALL offer "Open", which opens the screenshot in the app the operating system associates with its type, "Show in Finder" on macOS or "Show in File Explorer" on Windows, which opens its folder with the file selected, and "Copy path", which places its full path on the clipboard as text.

#### Scenario: Show in Finder

- **WHEN** the user runs "Show in Finder" on a screenshot on macOS
- **THEN** Finder opens the screenshot's folder with that file selected

#### Scenario: Show in File Explorer

- **WHEN** the user runs "Show in File Explorer" on a screenshot on Windows
- **THEN** File Explorer opens the screenshot's folder with that file selected

#### Scenario: Copy path

- **WHEN** the user runs "Copy path" on a screenshot
- **THEN** the clipboard holds the screenshot's full path as text

### Requirement: A screenshot can be moved to the trash

Each tile's action panel SHALL offer "Move to Trash" on macOS or "Move to Recycle Bin" on Windows. It SHALL move the file there without asking, since it can be put back, and SHALL show the grid again without that screenshot. A screenshot that cannot be moved SHALL stay in the grid with a failure ticker.

#### Scenario: Move to Trash on macOS

- **WHEN** the user runs "Move to Trash" on a screenshot on macOS
- **THEN** the file is in the Trash and can be put back from there
- **AND** the grid no longer shows it

#### Scenario: Move to Recycle Bin on Windows

- **WHEN** the user runs "Move to Recycle Bin" on a screenshot on Windows
- **THEN** the file is in the Recycle Bin and can be restored from there
- **AND** the grid no longer shows it

#### Scenario: The move fails

- **WHEN** a screenshot cannot be moved to the trash
- **THEN** the failure ticker reads "Couldn't move the screenshot to the Trash" on macOS or "Couldn't move the screenshot to the Recycle Bin" on Windows
- **AND** the screenshot stays in the grid

### Requirement: The latest screenshot can be pasted or copied without a view

`Paste Latest Screenshot` and `Copy Latest Screenshot` SHALL act on the most recently modified screenshot across all screenshot folders, exactly as the grid's Paste and Copy act on a chosen one. When there is no screenshot, they SHALL fail with "No screenshots found in your screenshot folders" and leave the clipboard unchanged.

#### Scenario: Paste the latest screenshot from a hotkey

- **WHEN** the user takes a screenshot, focuses a chat app's message field, and presses the hotkey bound to `Paste Latest Screenshot`
- **THEN** that screenshot is pasted into the message field

#### Scenario: Copy the latest screenshot

- **WHEN** the user invokes `Copy Latest Screenshot`
- **THEN** the clipboard holds the newest screenshot as image and file
- **AND** the notice reads "Copied to clipboard"

#### Scenario: No screenshot to act on

- **WHEN** the user invokes `Paste Latest Screenshot` and the screenshot folders hold no screenshots
- **THEN** the failure reads "No screenshots found in your screenshot folders"
- **AND** the clipboard is unchanged
