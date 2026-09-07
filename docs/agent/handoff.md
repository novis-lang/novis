# Handoff

## State

**Goal 13 is whole — both of stage 4's checks are green.** `python tools/dossier.py --only
lang:expressions/the-pipeline-operator --gate` says nothing is owed, and `python tools/reference.py
--check` says `docs/novis.md` is current at 286 of 286 examples. The operator itself shipped in
stage 2: `parse_pipe` at `crates/nvs-syntax/src/parser/expr.rs:559` parks the left side and
substitutes it into the one `$_`, and `E0129`/`E0130`/`E0131` hold `rule:expressions/pipeline-hole-once`.
Stage 3's four PHP 8.6 refusals are pinned under `tests/conformance/reject/php86/`.

**What the attack found, and it is not the pipeline's bug.** A `|>` chain is a *loop* in the parser,
so its stages are charged nothing against `MAX_RECURSION_DEPTH`, while the nested spelling it stands
for is refused at 96 levels with `E0108`. A 64-stage chain therefore builds a tree that overflows
`target/debug/nvs.exe`'s stack — but so does a 400-term `$n + 1 + 1 …`, because every
left-associative tier has that shape, and `target/release/nvs.exe` walks both. The hostile case sits
at 48 stages, which both builds walk, because what it is written to break is the parser's single
hole slot — pipelines nested in a right side, under a closure, under a `match` arm — and not the
debug build's stack. The depth gap is the next group.

## Next group

**The parser's recursion guard against tree depth, one file set:**
`crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/parser/mod.rs`,
`crates/nvs-syntax/src/parser/tests/expr.rs`.

- [ ] **A `|>` chain's stages are charged to the recursion guard**, so the operator cannot build a
      tree the spelling it stands for is refused for — `rule:expressions/pipeline-substitution` is
      the equivalence that makes the asymmetry a bug rather than a difference.
      `crates/nvs-syntax/src/parser/expr.rs:559` is `parse_pipe`'s loop, and
      `crates/nvs-syntax/src/parser/mod.rs:441` is `enter_recursive`, whose pairing with
      `exit_recursive` a loop has to hold open across the whole chain rather than per iteration.
- [ ] **The same bound at the shared left-associative helper**, which is where the general form of
      the gap lives: `crates/nvs-syntax/src/parser/expr.rs:424` is `parse_left_assoc`, and a
      400-term `+` chain overflows the debug build today. One case at the helper, not one per tier —
      `crates/nvs-syntax/src/parser/tests/expr.rs:1` is where the reject test goes.

## Backlog

- The three `expressions/pipeline-*` rules are `status: designed` in `docs/rules/expressions.json`
  though the tree ships all three — `docs/agent/conventions.md` § *A rule fragment*.
- ADR 0124's four refusal rules are `designed` for the same reason, and landed in stage 3.
- Every other `#` heading in `docs/reference/lang/` still owes its four proofs; the pipeline is the
  first `lang:` feature with any — `docs/examples/README.md`, `tests/hostile/README.md`.
- `docs/perf/members.ndjson` now carries a `lang:` row; the report's grouping has never been looked
  at with one in it — `benches/members/README.md`.
