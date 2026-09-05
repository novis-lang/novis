# Handoff

## State

**Goal 21 — The gaps no goal owned — has just started; nothing of it has landed yet.** Goal 6's whole
list is this goal's Stage 1 floor.

Every item is a gap already written down in the module that owns it, and the module doc is the design:
nothing here is a new decision, and `21-carried-gaps.md` § *Standing decisions* answers the four that
were left open (the wildcard's matching rule, what `nvs check` does with a broken `nvs.toml`, where an
`open`'s pool bounds come from, and that spec § 12's two decoders answer `bytes`). **Do not re-derive
any of those.** The audit that produced this goal is in `docs/agent/carried-gaps.md`, which is also
where a gap this goal does *not* take is recorded — that file survives a goal switch and this one does
not.

## Next group

**Stage 2: the keystone — an outstanding key names a goal that is still on the chain.** One file set:
`crates/nvs-stdlib/tests/spec_registry_coverage.rs` and the three `spec-*-outstanding.txt` lists beside
it, plus `docs/agent/carried-gaps.md`.

- [ ] **An outstanding key carries its owner in a column, not in a comment** —
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:583` is where the walk excludes § 13 and is
      the function that parses a key today. `§18 stream  # 21` is the shape. Once this box is ticked a
      key names the goal that will strike it, in a field a program reads.
- [ ] **The test fails on an owner that is not a live `[[goal]]`** — read out of
      `docs/agent/goals/chain.toml`. This is the whole point of the stage: `§18 stream` has been
      grouped under "goal 5's" since goal 5 went green on 2026-08-31, and nothing noticed.
- [ ] **Seed every one of the fifteen keys with an owner.** Two are this goal's (`§18 stream`,
      `§18 streamAs`); five are goal 17's (`§15 Request::clientIp`/`host`/`scheme` by that goal's own
      § 2, `§15 Response::html`/`sendFile`); the eight §§ 16–17 classes in
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` have **no owner on the chain** —
      record them in `docs/agent/carried-gaps.md` and leave the scheduling to the user rather than
      inventing an entry.

## Backlog

- Stage 3 (grants) is the next cheapest group and shares no files with stage 2:
  `crates/nvs-config/src/capability.rs:269`, `crates/nvs-cli/src/main.rs:623` and `:695`,
  `crates/nvs-types/src/check.rs:108`. Two items, both ADR 0067, both priority 1.
- Stage 4 (the array-closed cycle) is one file — `crates/nvs-runtime/src/object.rs:1816` and `:1658` —
  and its check is the WSL valgrind leg rather than stdout.
- Everything this goal does **not** take is in `docs/agent/carried-gaps.md`, not here: that file is
  read by the next goal and this section is overwritten by the next goal switch.
- When this goal's last check goes green the driver takes goal 22 — the on-disk artifact cache.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
