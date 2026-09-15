# Handoff

## State

**Goal 59 — every class M8 names is as deep as its spec section — has just started; nothing of it has landed yet.** Goal `m8-db-queue`'s whole list is this goal's Stage 1 floor.

Every design call a stage reaches is settled in the goal's § *Standing decisions*. That covers the
wire forms, the reflective call site, `allocate`'s rounding, the CSV spelling, where the metrics
registry moves, and what the one record decides. A session applies those decisions and does not
re-open them. The goal's one new record is stage 2's, on `array<T>` variance; no other stage opens a
number. Every check runs locally, because CI is not running.

## Next group

**Stage 0: the catch-up** — one file set: the `# Known gaps` blocks and the two ratchet files the goal
re-points, and nothing else. Doc edits only, so one session takes all three.

- [ ] **Re-point owners at `m8-stdlib-depth`** — `crates/nvs-stdlib/src/heap.rs:47`,
      `crates/nvs-stdlib/src/cldr.rs:165`, `crates/nvs-stdlib/src/cldr.rs:183`,
      `crates/nvs-stdlib/src/lib.rs:152`, `crates/nvs-stdlib/src/ast.rs:64`,
      `crates/nvs-stdlib/src/json.rs:143`,
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:16`,
      `crates/nvs-stdlib/tests/migration-members-outstanding.txt:22`, and the owner cell at
      `docs/agent/carried-gaps.md:62`. Done when `python tools/owners.py` lists every one of them under
      this goal.
- [ ] **Strike CLDR gap 3 into prose** — `crates/nvs-stdlib/src/cldr.rs:166`: the twenty languages are
      carried, and `crates/nvs-stdlib/src/cldr.rs:2812`'s test holds it. The boundary sentence joins
      the roster's prose, and the item leaves the block.
- [ ] **Rewrite `Core\Out`'s gap** — `crates/nvs-stdlib/src/out.rs:36`. `Text::plain` and `styled`
      exist (`crates/nvs-stdlib/src/cli.rs:1945`); what is missing is an instance member to read a
      `Text`, and stage 8 is what closes it.

## Backlog

- Stage 2 (the record, `crates/nvs-types/src/expr/assign.rs:60`) is its own session; it shares no
  file with stage 0.
- Stages 3 and 4 share nothing either. Stage 4 and stage 13's guard share
  `benches/abi-probe/tests/perf_guards.rs`, so a session holding that file cheaply can take both
  guards once `spawn` exists.
- When this goal's last check goes green the driver takes goal `unowned-closures`.
