# Handoff

## State

**Goal 6, M7 — stage 9's `nvs-stdlib (every member row is real)` check is green.** Both tests it
names exist in `crates/nvs-stdlib/tests/spec_registry_coverage.rs` and pass, and the file's own
module doc § *The third walk reads the other spec file* owns why they are there rather than in
`conformance_coverage.rs`. `cargo test -p nvs-stdlib --test spec_registry_coverage` is 7 passing.

**The parity table's coverage half was already closed** — `python tools/check-migration.py --min 100`
prints `100% covered` over 1152 classified rows, so that stage-9 check passes too. What the two new
tests add is the other half of the same claim: every `member` row's `Core\X::member` spelling
resolves to a registered member, and every registered one is called by a conformance case.

**Five members are owed and listed**, in `crates/nvs-stdlib/tests/migration-members-outstanding.txt`:
`Core\Db\Queryable::stream` (already `§18 stream` in the part-two ratchet — one slice strikes both),
`Core\Os::hostname`/`::memoryUsage`/`::pid` (no module in the crate at all), and
`Core\Process::spawn` (the class is registered and `run` is on it). The list only shrinks.

**Stage 9 has two checks left, and neither is a test to write beside another.** The
`tools/bench.py --serve-vs-fpm --record benches/serve.json` command check names a flag `tools/bench.py`
does not have and an artifact `benches/` does not hold; the `differential` suite's `min_passing = 275`
stands against 256 cases on disk, all of them passing.

## Next group

**Nineteen differential cases, to carry `min_passing = 275`.** The file set is
`tests/differential/core/` and `tests/differential/lang/`, and the check they answer is the
`differential` `nvs-suite` row of stage 9 in `docs/agent/loop-goal.toml`. `python tools/gaps.py` ranks
the PHP twins with no oracle case, so the picking is a tool call rather than a survey. Every case is
`--ORACLE--`, never `--EXPECT--` — `docs/agent/conventions.md` § *A `.nvst` test case* says why, and
PHP is on `PATH` on this box. The three anchors below are existing cases to read for shape, not files
to edit: `tests/differential/core/` already holds 181 cases and `tests/differential/lang/` 53.

- [ ] **Seven `Core\Arr` twins** — the members `gaps.py` names first, as
      `tests/differential/core/arr-chunk-matches-array_chunk.nvst:1`. One case per member, each asking
      the question its PHP twin answers differently at a boundary rather than in the middle.
- [ ] **Seven `Core\Str`/`Core\Num` twins** — same shape, same directory, as
      `tests/differential/core/str-chunk-matches-str_split-and-chunk_split.nvst:1`.
- [ ] **Five language-level divergences** — `tests/differential/lang/`, as
      `tests/differential/lang/a-foreach-by-reference-matches-phps.nvst:1`.

## Backlog

- `tools/bench.py` has no `--serve-vs-fpm` and `benches/` no `serve.json`; stage 9's command check in
  `docs/agent/loop-goal.toml` names both, and M7's *Verify* paragraph is what it is asserting.
- `Core\Os` has no module in `nvs-stdlib`; four PHP names point at it — see the ratchet file above.
- `Core\Db\Queryable::stream`/`streamAs` are owed by two ratchets at once, `§18 stream` and
  `Core\Db\Queryable::stream`; `docs/spec/01-core-library.md:1181` is the row.
