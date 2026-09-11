# Handoff

## State

**Goal `event-streams` is met — every stage's checks are green.** Stage 6's last red was the spec
rows: `docs/spec/01-core-library.md` § 16 now carries `Core\Sse`, `Core\Sse\Message` in the class
table `every_part_two_spec_class_is_registered` walks, and § 15's `Core\Response` bullet names
`stream(string $contentType)` beside the other body members, which is what the § 14-19 member walk
reads. Both are registered already, so neither ratchet file needed a line — and neither row is read
vacuously: renaming each to a bogus spelling fails its walk by name.

**The fifth check could never have gone green as written.** It named the test *target*
`spec_registry_coverage`, and `cargo-named` looks a name up in the run's output while
`tools/loop.py:2067`'s `crate_tests` runs each test executable directly, so no target name is ever
printed. It now names `every_part_two_spec_class_is_registered` and
`every_part_two_spec_member_is_registered`, in `docs/agent/loop-goal.toml` and in
`docs/agent/goals/42-event-streams.toml` both. The playbook bullet is the general form.

`Core\Socket::receive` is still inside `Ctx::deliver`'s known gap.

## Next group

**No stage of `event-streams` is open; this is the gap its spec row exposed** — one file set:
`docs/spec/01-core-library.md` and the class ratchet beside it under `crates/nvs-stdlib/tests/`.
Prose over landed behaviour, read by `cargo test -p nvs-stdlib` and nothing else.

- [ ] **`Core\Socket`'s § 16 row** — `docs/spec/01-core-library.md:1171` is `Core\Net`'s row, and
      the WebSocket door has none of its own anywhere in the file, so the class walk never asks
      about `crates/nvs-stdlib/src/socket.rs:222`'s `Core\Socket` or its
      `crates/nvs-stdlib/src/socket.rs:474` `Core\Socket\Message`. Write it as the sibling row
      directly above it is written — the surface, the refusals, then the ADR column.
      `rule:concurrency/two-doors-one-isolate`.
- [ ] **Whatever else §§ 16-17 never named** — `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:16`
      lists `§16 Core\Metrics` as the one outstanding class, which is only the classes the spec
      *does* name; a registered class the spec names nowhere is invisible to that gate entirely.
      Diff `registry::CLASSES` against the spans in §§ 16-17 and add a row per class that has none.

## Backlog

- `§15 Response::html` and `§15 Response::sendFile` are still unregistered ratchet lines, owned by
  goal `test-request` — `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:29`.
- `§16 Core\Metrics` has no owning goal — `docs/agent/carried-gaps.md` § *Unowned*.
- `docs/spec/01-core-library.md` §§ 16-17 have no member-level gate at all, by design —
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs:850` says what that costs.
