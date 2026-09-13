# Handoff

## State

**Goal `gap-register` — stages 1–3 green, stage 4 open.** `python tools/owners.py` now names every
module-doc heading that records owed work outside a `# Known gaps` block and counts them as
`sections outside Known gaps: N` (`tools/owners.py:164` is the pattern, `:379` the walk, `:701` the
line). The stage 4 check wants `0` (`docs/agent/loop-goal.toml:10111`); it stands at **18**.

- The goal's stage 4 authored a list of six from a substring sweep. The tool's pattern is
  word-bounded and spans `not … yet`, so it drops the false hits and finds the `# What is not here
  yet` headings that sweep could not see. The remaining 18 are the work, and `owners.py` prints them.
- Five sections are moved: `crates/nvs-stdlib/src/test.rs`, `crates/nvs-cli/src/runner.rs`,
  `crates/nvs-render/src/lib.rs`, `crates/nvs-server/src/schedule.rs` (new block, three items), and
  the two `nvs-types` headings whose sections owed nothing and only said they did.
- Owners are evidence, not guesses: `docs/decisions/0079.md:872` puts § 2's parallelism at M5 (goal
  `m5-proofs`) and § 22's output at the M4S tail, which goal `m8-stdlib-depth` § *Standing decisions*
  claims; `docs/decisions/0092.md:439` puts the compiler diagnostic's producer at M10.
- The gate is unchanged and green: `--check --untagged-is-an-error --reasons` passes, and
  `--deferrals` accepts the new `M10` tag against `docs/plan/m10.md`.

Nothing is blocked. `[context] modules` printed no crate module doc, which is the file kind this
stage edits; the driver's sweep picks up what these commits touched.

## Next group

**Stage 4 continued: `nvs-db`'s five sections** — one file set: `crates/nvs-db/src/`, whose gaps M8's
database half owns (goal `m8-db-queue`, `docs/agent/goals/58-m8-db-queue.md`). The rule is the goal's
stage 4 and `tools/owners.py`'s module doc § *A heading is not a register*.

- [ ] **`crates/nvs-db/src/tds/mod.rs:86` — `# The one gap` becomes the block it is already
      written as**, and `crates/nvs-db/src/tds/mod.rs:54` loses its `what is not yet` clause. The
      refused `bytes` bind is the item; this file has no `# Known gaps` block today.
- [ ] **`crates/nvs-db/src/lib.rs:100` — what each of the five wire implementations still owes**
      moves out of the prose into a new `# Known gaps` block, one numbered item per driver, and the
      heading becomes what is here.
- [ ] **`crates/nvs-db/src/span.rs:30` and `crates/nvs-db/src/ddl.rs:45`.** Span's section prices what
      it holds (`rule:programs/memory-priority` § *Say what you spend*, which is why the section
      stays) and names one capability no set can express yet — that half is the gap.
      `crates/nvs-db/src/ddl.rs:45` says a limit is *not* a gap here, so only its wording changes; its
      block is `crates/nvs-db/src/ddl.rs:73`.

## Backlog

- 13 sections outside a block in eight other crates — `python tools/owners.py` prints them with
  anchors; `crates/nvs-stdlib/src/http.rs:102` and `crates/nvs-server/src/serve.rs:32` are the two
  whose prose argues the omission is deliberate.
- Stage 5: `python tools/plan.py --past`, reading `owners.py --json` (goal prose stage 5).
- Stage 6: `tools/brief.py:471-483` routes `gap`, `owner` and `register` to `tools/owners.py`.
- 21 items are still tagged to a milestone the program has passed; `--past-is-an-error` names them
  and the closure goals empty them (goal `gap-register` § *Standing decisions*).
