# Handoff

## State

**M4's Stage 8, with the refusal ceiling at 4 — and one red acceptance check that is not a
missing test.** The tree is at 868 conformance plus 189 differential. Nothing is blocked.

`abi-probe [9 guards]` has failed after sessions 0055, 0056 and 0057.
`a_class_without_a_property_observer_costs_nothing_extra` reports 9 emitted calls where ADR 0014
§ 4 wants 0. **It is the guard that is wrong, not the lowering** — that judgement is this
session's, made from the IR and confirmed by running the shape, and the next session's job is to
land it, not to re-derive it:

* Each extra `$c->n = $c->n + 1` grows `Cell::plainOne`'s IR by exactly `BinOp`, `ConstInt`,
  `FieldGet`, `FieldSet`, `StmtMarker` and one **`Release`** of the receiver.
* Those releases are not on the hot path and are not a double release. `Cell::plainOne` ends with
  `BlockId(4) Return`, `BlockId(5) Propagate` and `BlockId(6) Propagate`, each holding one
  `Release { operand: $c }` — three **mutually exclusive** edges, one per fallible instruction's
  landing block, exactly one of which runs. A program of that shape run under `nvs run` prints
  the right answer, exits 0, and reads its `string` field back afterwards.
* The loop body itself, `BlockId(3)`, holds `FieldGet`/`BinOp`/`FieldSet` and **no call at all**.
  ADR 0014 § 4's claim is intact.
* What broke is `emitted_calls` at `benches/abi-probe/tests/perf_guards.rs:815`: it scans the
  whole function's disassembly, so a cold landing block counts as per-access cost. That became
  wrong when `bba9818` gave every status-returning instruction a landing block.

Two things landed here that are not the fix. `tools/orient.py` now prints the driver's last
acceptance verdict in the RUN section, and inlines `session.py --template` so the tail is two
calls rather than three. `tools/loop-stats.py` counts shell-carried file writes and treats them
and `splice.py` as edits, which moved the run's fixed cost from 49% of calls to 43% — the old
figure was counting a heredoc session's work as orientation.

## Next group

**Make the guard measure the hot path, then decide what the cold edges owe.** File set:
`benches/abi-probe/tests/perf_guards.rs` (`emitted_calls` at :815, the slope at :916, the
assertion at :941), `docs/adr/0014-*.md` § 4, and `crates/nvs-ir/src/lower/stmt.rs:1108-1130` if
the answer turns out to need the lowering after all.

- [ ] **Count calls on the path that runs, not in the whole function.** `emitted_calls` takes a
      function's whole disassembly; the guard wants the blocks a straight-line execution passes
      through. Excluding blocks terminated by `Terminator::Propagate` is the smallest change that
      says what ADR 0014 § 4 actually claims. The failure message already names the IR slope, so
      the next failure will say which instruction grew rather than only how many calls did.
- [ ] **Record the judgement in ADR 0014 § 4**: an access on an observer-free class is a direct
      field load or store *on the path that runs*, and a landing block releasing the receiver is
      not a dispatch the access asked for. One paragraph, not a new ADR — this is the goal's
      § *Standing decisions* "decide and record" case.
- [ ] **Keep the second half honest.** `hooked_extra > 0` at :951 is what stops the guard passing
      vacuously; whatever `emitted_calls` becomes, the hooked slope must still show the calls an
      opt-in costs.

## Backlog

- The 4 refusal sites: item 16's lowering half (`call.rs`, named/spread arguments), item 25's
  `object` representation arm, item 4's `stmt.rs` declaration catch-all. `python tools/holes.py
  --item N` prints any of them, and `refusals.rs`'s `CEILING` ratchets down with them.
- **At the M4 → M4B boundary, not before:** the pack is 72 KB, of which the traps are ~36 KB and
  the ADR sections ~17 KB. `0035 §4`, `0031 §2` and `0036 §2/§4` are Stages 3 and 5, both landed,
  and there are 0 named cases left to write, so the `Core`-era case-writing bullets no longer
  earn their place. Re-run `python tools/playbook.py --goal --min 3` against the current module
  list. Then re-measure with `loop-stats.py` and only then revisit the 120k slice gate in
  AGENTS.md — sessions are ending at ~110k against a 200k ceiling, so there is room, but that
  number is taken from the measurement and not from this bullet.
