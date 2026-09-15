## Purpose

Checks that run on the developer's machine at commit and push time, so formatting and lint failures are caught and fixed locally instead of in CI.

## ADDED Requirements

### Requirement: Committing Rust source formats it
The system SHALL format every staged Rust file with the project's rustfmt settings when a commit is created, stage the formatted result, and let the commit proceed with the formatted content. Unstaged edits in the working tree MUST be preserved unchanged.

#### Scenario: Staged Rust file is misformatted
- **WHEN** the developer commits a staged `.rs` file whose formatting differs from rustfmt output
- **THEN** the committed file matches rustfmt output and `cargo fmt --check` passes on the resulting commit

#### Scenario: Unstaged edits survive the commit
- **WHEN** the developer commits with a file that has both staged and unstaged hunks
- **THEN** after the commit the unstaged hunks are still present in the working tree, unformatted, and not part of the commit

#### Scenario: Commit without Rust files
- **WHEN** the developer commits only non-Rust files, such as Markdown or Svelte
- **THEN** no Rust tooling runs and the hook adds under 1 second to the commit

### Requirement: Pushing runs the CI lints
The system SHALL run clippy with `--all-targets` and warnings denied, and the frontend type check, before a push, using the same flags as the CI workflow, and SHALL abort the push when either fails.

#### Scenario: Clippy warning present
- **WHEN** the developer pushes a commit that produces a clippy warning under `--all-targets -- -D warnings`
- **THEN** the push is aborted and the warning is printed in the terminal

#### Scenario: Frontend type error present
- **WHEN** the developer pushes a commit for which `svelte-check` reports an error
- **THEN** the push is aborted and the error is printed in the terminal

#### Scenario: Clean tree
- **WHEN** clippy and `svelte-check` both pass
- **THEN** the push proceeds without further prompts

### Requirement: Checks run on demand
The system SHALL let the developer run the same checks against the working tree without committing or pushing, in a check-only mode and in a mode that applies available fixes.

#### Scenario: Check-only run
- **WHEN** the developer runs the check command on a tree with a misformatted Rust file
- **THEN** the command exits non-zero, names the file, and modifies nothing

#### Scenario: Fix run
- **WHEN** the developer runs the fix command on a tree with a misformatted Rust file
- **THEN** the file is rewritten to rustfmt output and left unstaged

### Requirement: Setup is one pinned command per clone
The system SHALL install the hook runner through the project's mise configuration at a pinned version and SHALL require exactly one documented command per clone to activate the hooks. Hooks MUST resolve the mise-managed tools even when the shell has not activated mise.

#### Scenario: Fresh clone on Windows
- **WHEN** the developer runs `mise install` and then the documented hook install command in a fresh clone on Windows, then commits a misformatted Rust file from PowerShell
- **THEN** the file is formatted before the commit lands

#### Scenario: Fresh clone on macOS
- **WHEN** the developer runs `mise install` and then the documented hook install command in a fresh clone on macOS, then commits a misformatted Rust file
- **THEN** the file is formatted before the commit lands

#### Scenario: Hook runner not installed
- **WHEN** the hooks have not been installed in a clone
- **THEN** commits and pushes behave exactly as before this change and CI remains the gate
