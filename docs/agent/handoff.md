# Handoff

## State

**Goal `m8-db-queue`, stage 0 (the catch-up) is done; no stage-2-and-later work has started.** Every
`— owner:` tag on a gap this goal will close now reads `m8-db-queue`, and the three gaps the goal's
§ *Standing decisions* reserves for goal `unowned-closures` are untouched.

Stage 0 item 4 needed no edit: `crates/nvs-stdlib/src/queue.rs` gap 1 already claims only `limits` and
`grants`, with no secret refusal left in it. The goal file's stage 0 anchors are one to three lines off
throughout and two of its tags had already been corrected — re-grep rather than trusting them.

Stage 1 is goal `m7-server-surface`'s carried floor and is the driver's to run, not a session's.

## Next group

**Stage 2: the record** — one file set: a new record under `docs/decisions/`, three new fragments and
four edited ones under `docs/rules/`, then `python tools/rules.py --render`. The record's body is
`docs/agent/loop-goal.md:66`'s four decisions argued from § *Standing decisions*; no code is touched.

- [ ] **Write the record, claiming the next free number**, which was `0187` when this session read
      `docs/decisions/` — re-derive it immediately before creating the file, because the chain's other
      sessions derive the same answer from the same directory. Its four decisions are listed at
      `docs/agent/loop-goal.md:71` and argued at `docs/agent/loop-goal.md:251`
      (`rule:core-classes/db-streaming` is the one it reaches past).
- [ ] **Create the three `designed` rules** the table at `docs/agent/loop-goal.md:85` names —
      `core-classes/a-stream-parks-its-read-on-the-connection`,
      `core-classes/server-version-is-what-the-server-said` and
      `core-classes/a-unique-key-reads-nulls-as-distinct` — as fragments under
      `docs/rules/core-classes/`, each citing the new record as its `because`, then render.
      `docs/rules/core-classes/db-streaming.md:11` is the shape a `designed` fragment reads as.
- [ ] **Edit the four existing fragments** `docs/agent/loop-goal.md:93` lists:
      `docs/rules/core-classes/db-streaming.md:11`'s *Not shipped whole* paragraph goes,
      `docs/rules/core-classes/queue-storage-is-a-table.md:16`'s SQL Server paragraph becomes a
      statement of fact, `docs/rules/core-classes/schema-plan.md:1`'s v1 exclusions admit the filtered
      index, and `docs/rules/concurrency/attempts-are-finite-and-a-dead-letter-is-kept.md:1` gains the
      column that carries `errors`. A fragment is edited in place, never overlaid
      (`docs/agent/doc-style.md` § *Edit the rule, never overlay it*).

## Backlog

- `crates/nvs-stdlib/src/db/row.rs:137` and `:206` both cite "`nvs_stdlib::db`'s own known gap 8", but
  that module doc carries four gaps — a stale cross-reference for stage 10's rewrite of `db/mod.rs`.
- `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:5` still uses `# gap-zero` as the
  illustrative owner in its format example; harmless, but it is now the file's only mention of it.
- `docs/agent/loop-goal.md` stage 0's anchors and two of its tag claims are stale as written; nothing
  reads that file mechanically, so this is a note rather than an edit.
