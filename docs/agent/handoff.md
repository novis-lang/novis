# Handoff

## State

**Goal `gap-register` — stages 1–4 and 6 green, stage 5 the only one open.** `python tools/owners.py`
reports `sections outside Known gaps: 0`, and `--check --untagged-is-an-error --reasons` passes with
`untagged: 0`, `broken-tag: 0`, `unreasoned: 0` over 173 tagged items.

- Stage 5's check fails as `plan.py: error: unrecognized arguments: --past` — the mode is still to be
  written, not a regression. Stage 6 already passes, so stage 5 is the whole of what is left.
- Four of the seven headings closed this session named work that had **landed**, not work owed, so
  they became prose: `sse.rs`'s door-one half, `http.rs`'s two absences, `dispatch.rs` and
  `script.rs`. The playbook bullet under *Tooling* is that trap.
- Three became real registers: `cache.rs` gap 1 (`unowned`), `openapi.rs` gaps 1–5 (`unowned`),
  `regions.rs` gap 1 (`M10`), `lib.rs` gap 1 (`M11`), and `budget.rs`'s bold `**Known gap.**` run is
  now a numbered item — which moved it to `crates/nvs-runtime/src/budget.rs:89` and re-pointed goal
  `unowned-closures`'s two anchors.
- `docs/agent/carried-gaps.md` § *Unowned* carries the two new reason bullets the `unowned` tags need.

Nothing is blocked. `[context] modules` still prints no crate module doc, which is the file kind this
stage edits.

## Next group

**Stage 5: a past milestone says whether it is complete** — one file set: `tools/plan.py`, reading
`tools/owners.py --json`. The specification is the goal's own § *Stage 5*
(`docs/agent/loop-goal.md:90`); the check wants `M0`, `M4S`, `M4B`, `M8` and
`past milestone(s) complete` in the output.

- [ ] **`python tools/plan.py --past` prints one line per pre-M9 milestone** — the goals carrying it
      and whether each has walked, the count any register still tags to it, and a last line
      `N of 11 past milestone(s) complete` — `tools/plan.py:759`, `tools/owners.py:770`.
- [ ] **`--sync` writes `done` into a complete milestone's `Carried by` cell** — only for one
      `--past` calls complete — `tools/plan.py:770`.
- [ ] **`--check` accepts `done` only for a milestone `--past` calls complete** — `tools/plan.py:767`.

## Backlog

- `crates/nvs-test/src/case.rs:351`'s `NOT_YET` reason names M6, a walked milestone; the assertion at
  `crates/nvs-test/src/case.rs:1232` reads `"M6"`, so both change together (that file's gap 1).
- `crates/nvs-stdlib/src/cache.rs:2009`'s `rediss://` refusal says which certificates this binary
  trusts "has no decision yet"; they are compiled in (`crates/nvs-stdlib/src/http/transport.rs:62`).
- 21 items are still tagged to a milestone the program has passed — stage 5's own subject.
- `docs/agent/goals/42-event-streams.md:13-17` still cites `serve.rs` and `registry.rs` line numbers
  from before the goal landed; a goal file describes the tree it was written against.
