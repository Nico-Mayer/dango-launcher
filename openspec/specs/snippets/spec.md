# snippets Specification

## Purpose

The built-in extension that keeps the pieces of text the user types again and
again, finds them by name from the launcher's main surface, and puts them into
whatever they were working in.

## Requirements

### Requirement: A snippet is created, edited, and removed from the launcher

The extension SHALL let the user create a snippet with a name and a template,
change either afterwards, and delete it, without leaving the launcher.

#### Scenario: Creating a snippet

- **WHEN** the user completes the create form with a name and a template
- **THEN** the snippet is stored
- **AND** it can be found in root search straight away

#### Scenario: A snippet needs a name and a template

- **WHEN** the user submits the create form with either one empty
- **THEN** the snippet is not stored
- **AND** the form says what is missing

#### Scenario: Editing a snippet

- **WHEN** the user edits a snippet's name or template and submits
- **THEN** the stored snippet reflects the change
- **AND** the old version is not kept

#### Scenario: Removing a snippet

- **WHEN** the user removes a snippet
- **THEN** it no longer appears in root search
- **AND** the list the user was looking at stays open

#### Scenario: A snippet whose template cannot be parsed is refused

- **WHEN** the user saves a snippet whose template cannot be parsed
- **THEN** it is refused with a message and nothing is stored

### Requirement: The create and edit forms show what a snippet will ask for

While the user is writing a snippet's template, the form SHALL show the
arguments that template will ask for when it is used, so that text which is a
placeholder by accident is visible while it is still being edited.

#### Scenario: A template with arguments shows them

- **WHEN** the user's template references two names that are not reserved
- **THEN** the form shows both as arguments the snippet will ask for

#### Scenario: The preview follows the template as it is edited

- **WHEN** the user changes the template
- **THEN** the arguments shown change to match

#### Scenario: A template with no arguments says so

- **WHEN** the user's template references only reserved names, or none
- **THEN** the form shows that the snippet will ask for nothing

#### Scenario: Pasted text that is a placeholder by accident is visible

- **WHEN** the user pastes text containing a doubled-brace expression from another templating system
- **THEN** the form shows the argument it would ask for
- **AND** the user can see it before saving

#### Scenario: A template that cannot be parsed says so while it is typed

- **WHEN** the template cannot be parsed
- **THEN** the form says so instead of showing arguments

### Requirement: Snippets are found in root search

Every snippet the user has created SHALL appear as a result in root search,
matched on its name, and SHALL be distinguishable from other results.

#### Scenario: A snippet is found by its name

- **WHEN** the user types part of a snippet's name
- **THEN** that snippet appears in the results

#### Scenario: A snippet shows what it is

- **WHEN** a snippet appears in the results
- **THEN** its name is shown along with an indication of what its text is

#### Scenario: Snippets do not crowd out applications

- **WHEN** the user types a query matching both an application and a snippet
- **THEN** both appear, ordered by the launcher's usual ranking

#### Scenario: Snippets stay inside the provider budget

- **WHEN** root search queries the snippets provider with 500 snippets stored
- **THEN** it answers within the 50ms provider budget

### Requirement: Confirming a snippet inserts its text

Confirming a snippet SHALL render its template and insert the result into the
application the user was in.

#### Scenario: A snippet with no arguments

- **WHEN** the user confirms a snippet whose template needs no arguments
- **THEN** the launcher closes and the rendered text appears in the frontmost application

#### Scenario: A snippet using the clipboard or the selection

- **WHEN** the user confirms a snippet whose template uses `clipboard` or `selection`
- **THEN** the inserted text contains the clipboard's text or the text that was selected

#### Scenario: A snippet declaring a caret position

- **WHEN** the user confirms a snippet whose template declares a caret position
- **THEN** the caret is left at that position in the inserted text

#### Scenario: Insertion is not possible

- **WHEN** the text cannot be inserted
- **THEN** the launcher stays open with a message explaining why
- **AND** nothing is inserted anywhere

### Requirement: A snippet with arguments asks before it inserts

A snippet whose template needs arguments SHALL ask the user for them, in the
order the template uses them, before rendering.

#### Scenario: The user is asked for arguments

- **WHEN** the user confirms a snippet whose template needs two arguments
- **THEN** a form is shown with one field per argument, in the order the template uses them

#### Scenario: Submitting the arguments inserts the text

- **WHEN** the user fills in the form and submits it
- **THEN** the template is rendered with those values and the result is inserted

#### Scenario: Abandoning the form inserts nothing

- **WHEN** the user dismisses the form without submitting
- **THEN** nothing is inserted
- **AND** the clipboard is unchanged

### Requirement: A snippet can be copied instead of inserted

Every snippet SHALL offer copying its rendered text to the clipboard as an
alternative to inserting it, for the cases where inserting is not wanted or not
possible.

#### Scenario: Copying a snippet

- **WHEN** the user chooses to copy a snippet rather than insert it
- **THEN** its rendered text is on the clipboard
- **AND** the launcher closes

#### Scenario: A copied snippet is the user's own copy

- **WHEN** the user copies a snippet's text
- **THEN** it is recorded in the clipboard history like any copy they made themselves

### Requirement: Snippets survive a restart

A snippet SHALL persist across restarts, unchanged in name and template.

#### Scenario: Snippets come back after a restart

- **WHEN** Dango is stopped and started again
- **THEN** every snippet the user created is still there with the same name and template

#### Scenario: A removed snippet stays removed

- **WHEN** a snippet is removed and Dango is restarted
- **THEN** it does not come back

### Requirement: A snippet may declare a keyword

A snippet MAY carry a keyword, which is what expands it in place. A keyword
SHALL be optional, SHALL be unique among snippets, and SHALL be refused for a
snippet whose template needs arguments.

#### Scenario: Creating a snippet with a keyword

- **WHEN** the user completes the create form including a keyword
- **THEN** the snippet is stored with it
- **AND** typing that keyword in any application expands the snippet

#### Scenario: A snippet without a keyword still works

- **WHEN** the user leaves the keyword empty
- **THEN** the snippet is stored
- **AND** it is still found and confirmed from root search

#### Scenario: Two snippets cannot share a keyword

- **WHEN** the user saves a snippet with a keyword another snippet already has
- **THEN** it is refused with a message naming the conflict
- **AND** neither snippet is changed

#### Scenario: A snippet that asks for arguments cannot have a keyword

- **WHEN** the user gives a keyword to a snippet whose template needs arguments
- **THEN** it is refused with a message explaining that expanding in place cannot ask for them
- **AND** the snippet can still be saved without a keyword

#### Scenario: Removing a keyword

- **WHEN** the user clears a snippet's keyword and saves
- **THEN** typing the old keyword no longer expands anything
- **AND** the snippet is still found in root search

#### Scenario: A keyword is shown where the snippet is

- **WHEN** a snippet with a keyword appears in root search
- **THEN** its keyword is visible, so the user can remember what to type

#### Scenario: Removing a snippet removes its keyword

- **WHEN** a snippet is removed
- **THEN** typing its keyword no longer expands anything
