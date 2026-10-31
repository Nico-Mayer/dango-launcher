## MODIFIED Requirements

### Requirement: Committing Rust source formats it

The system SHALL format every staged Rust file with the project's rustfmt
settings when a commit is created, stage the formatted result, and let the
commit proceed with the formatted content. Unstaged edits in the working tree
MUST be preserved unchanged.

#### Scenario: Staged Rust file is misformatted

- **WHEN** the developer commits a staged `.rs` file whose formatting differs from rustfmt output
- **THEN** the committed file matches rustfmt output and `cargo fmt --check` passes on the resulting commit

#### Scenario: Unstaged edits survive the commit

- **WHEN** the developer commits with a file that has both staged and unstaged hunks
- **THEN** after the commit the unstaged hunks are still present in the working tree, unformatted, and not part of the commit

#### Scenario: Commit without Rust files

- **WHEN** the developer commits only non-Rust files, such as Markdown or `.slint` files
- **THEN** no Rust tooling runs and the hook adds no noticeable time to the commit

### Requirement: Pushing runs the CI lints

The system SHALL run clippy with `--all-targets` and warnings denied before a
push, using the same flags as the CI workflow, and SHALL abort the push when it
fails. Because the build compiles the `.slint` files, an interface file that
does not compile SHALL fail the same check.

#### Scenario: Clippy warning present

- **WHEN** the developer pushes a commit that produces a clippy warning under `--all-targets -- -D warnings`
- **THEN** the push is aborted and the warning is printed in the terminal

#### Scenario: Frontend type error present

- **WHEN** the developer pushes a commit with a `.slint` file that does not compile
- **THEN** the push is aborted and the compiler error is printed in the terminal

#### Scenario: Clean tree

- **WHEN** clippy passes
- **THEN** the push proceeds without further prompts
