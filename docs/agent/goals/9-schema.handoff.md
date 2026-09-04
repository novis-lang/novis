# Handoff

## State

**Goal 9 — `Core\Db\Schema` — has just started; nothing of it has landed yet.** Goal 8's whole list is
this goal's Stage 1 floor, and **goal 5's two `nvs queue migrate` checks are inside it** — that is what
makes stage 7's retirement a behaviour-preserving refactor against frozen output rather than a rewrite
anyone has to trust.

The design is settled in the goal prose and is input to the ADR, not a question it reopens: a **closed
vocabulary** of tables, columns and indexes with no raw escape hatch; **convergence, never versioning**
(`Core` holds no notion of a migration, and [ADR 0082](../../adr/0082-the-first-party-framework.md) § 7
stays blocked); **live introspection and never a SQL parser**; **absence never destroys**; three grades
with an unknown grade grading *up*; and a plan whose every step carries complete executable SQL,
including the steps `apply` refuses.

## Next group

**Stage 2: the ADR, then the vocabulary** — one file set: `crates/nvs-db/src/schema.rs` (new), beside
`crates/nvs-db/src/sql.rs:72`.

- [ ] **The ADR, this goal's one slot and its first slice.** It closes
      [ADR 0067](../../adr/0067-core-db.md)'s *Revisiting* item — *"a portable `Core\Db\Schema`, if
      migration tooling in `nvs` itself needs it"* — takes over
      [ADR 0084](../../adr/0084-durable-background-jobs.md) § 2's schema, and carries the placement
      argument through [ADR 0051](../../adr/0051-standard-library-tiers.md) tests 2, 1 and 6. Take the
      number that is free when the slice starts.
- [ ] **The vocabulary in a new `crates/nvs-db/src/schema.rs`** — table, columns, primary key, unique
      constraints, indexes, and the column-type enum that is
      [ADR 0067](../../adr/0067-core-db.md) § 9's map in the write direction. The read direction is
      already on disk and tested, so the named test holding the two together is what keeps it one table
      used both ways.
- [ ] **The array round-trip** — `fromArray(toArray(x)) == x` over every construct. It is the property
      that makes a file, a builder and stage 4's `dump` one value rather than three.

## Backlog

- Stage 3 (four emitters, following `Dialect` and not `Driver`) shares stage 2's file set exactly and is
  the natural second slice if there is headroom. The MySQL traps are transcription rather than
  discovery — `crates/nvs-stdlib/src/queue.rs:296` already writes down all three.
- Stages 4 and 5 (the five introspectors, then the diff and the grades) are where the containers start
  mattering: stage 5's property — apply, introspect, empty plan — is asserted on every backend.
- Stage 6 crosses into `nvs-stdlib` and `nvs-cli` for the first time; stage 7 goes back to
  `crates/nvs-stdlib/src/queue.rs` alone.
- When this goal's last check goes green the driver takes goal 10 — ADR 0136's typed `callable`. The
  chain runs to goal 17; M4B is entries 12–15, and 16–17 are the request-body and test-request pair.
