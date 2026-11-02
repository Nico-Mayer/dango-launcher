# Design

## Context

See proposal.md - Why. Today the `key` string flows from `Hyperkey` in
`src/config/mod.rs` through `HyperkeyTrigger::from_key` (`src/platform/mod.rs`)
in the hyperkey start path in `src/app/mod.rs`. The platform code on both
sides only ever receives `HyperkeyTrigger::CapsLock`. The schema
(`src/config/schema.rs`) pins `key` to an enum of `capslock`/`caps`, and
`docs/config.md` is generated from the schema and checked by a test.

## Goals / Non-Goals

**Goals:**

- No config value can produce the "not a remappable key" failure.
- No change to `src/platform/macos/`, so the change is fully built and tested
  on Windows.

**Non-Goals:**

- Reworking how the hyperkey is started, reloaded, or reported.

## Decisions

**Keep `HyperkeyTrigger` with its single `CapsLock` variant.** The start path
builds `HyperkeySpec { trigger: HyperkeyTrigger::CapsLock, .. }` directly.
Rejected: deleting the enum and the `trigger` field. That removes a few more
lines but edits `src/platform/macos/hyperkey.rs`, which cannot be compiled from
Windows, for no behaviour gain. The enum also gives the type a name if another
trigger is ever added.

**A leftover `key` is ignored, not rejected.** Config parsing already ignores
unknown keys, and the schema flags them in the editor
(`openspec/specs/config-schema`: the runtime stays lenient). Rejected: failing
the hyperkey or the whole config on a leftover `key`. That would recreate the
trap this change removes, for a value that now has nothing to choose.

**The live-reload comparison drops the key.** The start path compares
`(shift, tap)` instead of `(key, shift, tap)` to decide whether to restart the
hyperkey. Nothing else changes there.

No user-visible text is added or changed: the only string removed is the
"is not a remappable key" log line.

## Risks / Trade-offs

- [A leftover `key: "f13"` silently gives CapsLock] → Accepted: the schema marks
  `key` as not allowed in the editor, and CapsLock is the only key the spec has
  ever allowed.
