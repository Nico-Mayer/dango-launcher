## Why

On Windows, launching File Explorer from root search fails with "That app is
no longer installed" and drops it from results, although it is installed. Its
shell entry's target is a shell namespace identifier of the form `::{GUID}`,
which the launcher mistakes for a drive path because its second character is
a colon. The path does not exist on disk, so the launch is judged stale and the
entry is removed. The next index rebuild brings it back, and the next launch
removes it again. Explorer is the one real application affected on the
author's machine, and it is also the one the author opens most.

More generally, a single guess about a path decides whether an entry is
deleted from the index. The index itself is rebuilt from the platform's own
list of applications, so that list, not a heuristic, should have the last word
on whether an application is gone.

## What Changes

- The Windows path check recognises a drive path only when it starts with a
  drive letter, a colon, and a separator, so shell namespace targets are no
  longer treated as files. An indexed application keeps a target only when
  it is a file path, so a shell namespace identifier is neither shown as a
  subtitle nor offered for reveal and copy.
- A failed launch no longer removes an entry on its own. The index is rebuilt
  from the platform first; the entry is reported as gone only when the rebuilt
  index no longer lists it, and otherwise the launch failure is shown and the
  entry stays.
- Confirming a result whose entry is missing from the index, for example
  because a rebuild replaced the index between search and Enter, rebuilds and
  retries the lookup before reporting the application as gone.

Milestone: a fix within M1 core, where the applications extension was built.
Platforms: the path bug is Windows only. The rebuild-before-remove rule lives
in the platform-neutral core and applies to both Windows and macOS.

## Non-goals

- Mapping individual shell launch error codes to distinct messages.
- Changing what the Windows indexer keeps or drops during enumeration.
- A reveal action for applications without a file target. Explorer keeps
  reporting that it has no file to reveal.
- Any change to the macOS indexer.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `applications`: "Discovery on Windows" gains a scenario for an application
  whose target is a shell namespace identifier. "The index stays fresh"
  replaces "Stale entry fails to launch" with removal confirmed by the
  platform, and adds a scenario for a launch that fails while the application
  is still installed. The hotkey requirement already defers to the launch
  rule for an entry that turns out to be gone, so it is not touched; it is
  also being modified by the in-progress `add-adaptive-ranking` change.

## Impact

- `src-tauri/src/platform/windows/apps.rs`: `is_file_path` and its tests.
- `src-tauri/src/extensions/applications/mod.rs` and `index.rs`: the launch
  failure path rebuilds and consults the index instead of calling `remove`.
- No change to the extension manifest, the view protocol, the config file, or
  stored data.
