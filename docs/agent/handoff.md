# Handoff

## State

Goal `m8-stdlib-depth`, **stage 0 — the catch-up — is done**, and nothing of stages 1–15 has
started. Doc and comment edits only: no member, no behaviour, one test's assert message.

`python tools/owners.py` puts 12 items under this goal and none under `gap-zero`; `untagged`,
`broken-tag` and `retired-owner` are all 0, and `past-milestone` is 16 (§ *Backlog*).

Two of stage 0's nine anchors were already correct on disk (`crates/nvs-stdlib/src/ast.rs:64`,
`crates/nvs-stdlib/src/json.rs:143`). `docs/agent/carried-gaps.md:62` resolved to the `array<T>`
row rather than a CLDR one — the file had lost rows since the goal was written — so that row and
`crates/nvs-stdlib/src/lib.rs` gap 4, which are one fact, both name this goal now. That is also the
right owner: stage 2's text hands the rulebook amendment to goal `plan-truth`, which is retired
(`docs/agent/goals/52-plan-truth.md` has no sibling `.toml`), and § *Standing decisions* gives this
goal a record for it.

## Next group

**Stage 2: `array<T>` is covariant in the checker and invariant in the rulebook** — one file set:
`crates/nvs-types/src/expr/assign.rs`, `rule:types/arrays`' fragment, and the two homes of the gap
that closes when they agree.

- [ ] **Confirm the covariance and pin it with a case** —
      `crates/nvs-types/src/expr/assign.rs:159-175` recurses element to element, with the
      copy-on-write soundness argument in its own comment (the goal's anchor, not re-checked this
      session). `rule:types/arrays`. The goal asks a session to confirm `Core\Arr::flip($stringArray)`
      compiles; if it does not, that is a gap in the code and is built here rather than recorded.
- [ ] **Amend `rule:types/arrays` to the covariant read, and open the record** —
      `crates/nvs-types/src/expr/assign.rs:228` still speaks of invariance as live, and the fragment
      carries records 0007, 0069, 0114, 0002 and 0159 asserting it. The reasoning to cite is the
      `Decided:` sentence at `crates/nvs-stdlib/src/lib.rs:151`. The next free record number was 0188
      at this commit; re-derive it before claiming one.
- [ ] **Strike the gap from both its homes** — `crates/nvs-stdlib/src/lib.rs:144` gap 4, and the
      `array<T>` row at `docs/agent/carried-gaps.md:54`.

## Backlog

- 16 `# Known gaps` items are tagged with a milestone the program has passed; 8 are in this goal's
  own files and owned by one of its stages — `crates/nvs-stdlib/src/ast.rs:46`, `cli.rs:137`,
  `csv.rs:128`, `debug.rs:81`, `decimal.rs:38`, `math.rs:33`, `reflect.rs:73`/`:78`/`:83`. Stage 0
  re-pointed `out.rs` alone, because only that one's prose was wrong as well. `python tools/owners.py`.
- CLDR's gap 4 is now gap 3. `docs/agent/goals/59-m8-stdlib-depth.md:202` (stage 11) and its
  `.toml`'s check comment at `:4320` still say "gap 4" and "gaps 2-4".
- `every_language_named_absent_in_the_gap_note_now_has_a_rule` outlives the note it is named for;
  `docs/agent/goals/59-m8-stdlib-depth.toml:4328` names it as a guard, so a rename is a two-file edit.
- Stage 4 and stage 13's guard share `benches/abi-probe/tests/perf_guards.rs`, so a session holding
  that file can take both guards once `spawn` exists. Stages 3 and 4 share nothing else.
- When this goal's last check goes green the driver takes goal `unowned-closures`.
