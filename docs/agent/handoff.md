# Handoff

## State

**Goal 6, M7 — stage 9's conformance check is green.** Its `cases` list named seven `.nvst` paths
under a `tests/conformance/http/` directory the corpus never adopted; the corrected list and the
reasoning for every row are in `docs/agent/loop-goal.toml`'s comment block directly above that check.
Three claims were already landed under the name the corpus took — the match a request carries, the
`404`/`405` pair, and the `echo`/typed-body refusal. Five cases were written for the list, all under
`tests/conformance/core/`, and the suite is **1548 passing, 0 failing**.

**Two of the five are pinned where a case can be held to them rather than where the claim is worded**,
and the playbook's *A `.nvst` case gets exactly one in-process request* bullet owns why: a session
surviving a core is asserted at the boot, and a deferred tree outliving its connection from the
caller's side.

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
PHP is on `PATH` on this box.

- [ ] **Seven `Core\Arr` twins** — the members `gaps.py` names first, as
      `tests/differential/core/arr-chunk-matches-array_chunk.nvst:1`. One case per member, each asking
      the question its PHP twin answers differently at a boundary rather than in the middle.
- [ ] **Seven `Core\Str`/`Core\Num` twins** — same shape, same directory, as
      `tests/differential/core/str-chunk-matches-str_split-and-chunk_split.nvst:1`.
- [ ] **Five language-level divergences** — `tests/differential/lang/`, as
      `tests/differential/lang/a-for-loop-matches-php.nvst:1`.

## Backlog

- `tools/bench.py --serve-vs-fpm --record benches/serve.json` — stage 9's other open check; the flag
  and the `benches/` artifact both have to be built. Owned by `docs/plan/m7.md`'s *Verify*.
- The `[context]` manifest printed nothing about the conformance corpus's own layout or about
  `crates/nvs-cli/src/runner.rs`'s in-process `answer`, both of which this session needed; a
  `modules` pattern for `nvs-cli/src/runner.rs` would have covered the second. Owned by
  `docs/agent/loop-goal.toml` `[context]`.
