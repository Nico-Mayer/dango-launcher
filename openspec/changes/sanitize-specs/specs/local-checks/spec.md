## MODIFIED Requirements

### Requirement: Setup is one pinned command per clone

The system SHALL install the hook runner through the project's mise
configuration at a pinned version and SHALL require exactly one documented
command per clone to activate the hooks. Hooks MUST resolve the mise-managed
tools even when the shell has not activated mise.

#### Scenario: Fresh clone on Windows

- **WHEN** the developer runs `mise install` and then the documented hook install command in a fresh clone on Windows, then commits a misformatted Rust file from PowerShell
- **THEN** the file is formatted before the commit lands

#### Scenario: Fresh clone on macOS

- **WHEN** the developer runs `mise install` and then the documented hook install command in a fresh clone on macOS, then commits a misformatted Rust file
- **THEN** the file is formatted before the commit lands

#### Scenario: Hook runner not installed

- **WHEN** the hooks have not been installed in a clone
- **THEN** commits and pushes run no checks and CI remains the gate

## REMOVED Requirements

### Requirement: Pushing runs the CI lints

**Reason**: Replaced by "Pushing runs the CI lints and the Slint compiler". The "Frontend type error" scenario is renamed for the Slint UI. OpenSpec cannot rename or drop a
scenario inside a MODIFIED block.

**Migration**: None. The rule is unchanged under the new header.

## ADDED Requirements

### Requirement: Pushing runs the CI lints and the Slint compiler

The system SHALL run clippy with `--all-targets` and warnings denied before a
push, using the same flags as the CI workflow, and SHALL abort the push when it
fails. Clippy builds the crate and the build compiles the `.slint` files, so a
`.slint` file that does not compile SHALL fail the same check.

#### Scenario: Clippy warning present

- **WHEN** the developer pushes a commit that produces a clippy warning under `--all-targets -- -D warnings`
- **THEN** the push is aborted and the warning is printed in the terminal

#### Scenario: Slint file does not compile

- **WHEN** the developer pushes a commit with a `.slint` file that does not compile
- **THEN** the push is aborted and the compiler error is printed in the terminal

#### Scenario: Clean tree

- **WHEN** clippy passes
- **THEN** the push proceeds without further prompts
