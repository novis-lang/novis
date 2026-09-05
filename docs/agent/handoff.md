# Handoff

## State

**Goal 6, M7 — stage 9's memory floor is closed**, and it is checked at `-p nvs-host` rather than
`-p nvs-server`. `a_request_that_builds_cycles_returns_its_bytes_at_teardown` and
`live_bytes_are_flat_across_a_cycle_building_soak` are both in
`crates/nvs-host/src/isolate.rs`'s test module, over one shared fixture; each carries in its own doc
comment why it is not at the door, and so does the check's comment block. The short of it: a request
*is* the isolate, but the cycle has to be built by hand, `NvsObj::new` is `unsafe`, and
`crates/nvs-server` inherits the workspace's `unsafe_code = "forbid"`.

**Stage 9 has two checks left, and neither is a test to write beside another.** The
`tools/bench.py --serve-vs-fpm --record benches/serve.json` command check names a flag `tools/bench.py`
does not have and an artifact `benches/` does not hold; the `differential` suite's `min_passing = 275`
stands against 256 cases on disk, all of them passing.

## Next group

**Nineteen differential cases, to carry `min_passing = 275`.** The file set is
`tests/differential/core/` and `tests/differential/lang/`, and the check they answer is
`docs/agent/loop-goal.toml:4087`. `python tools/gaps.py` ranks the PHP twins with no oracle case, so
the picking is a tool call rather than a survey. Every case is `--ORACLE--`, never `--EXPECT--` —
`docs/agent/conventions.md` § *A `.nvst` test case* says why, and PHP is on `PATH` on this box.

- [ ] **Seven `Core\Arr` twins** — the members `gaps.py` names first, as
      `tests/differential/core/arr-chunk-matches-array_chunk.nvst:1`. One case per member, each asking
      the question its PHP twin answers differently at a boundary rather than in the middle.
- [ ] **Seven `Core\Str`/`Core\Num` twins** — same shape, same directory, as
      `tests/differential/core/arr-count-matches-count.nvst:1`.
- [ ] **Five language-level divergences** — `tests/differential/lang/`, as
      `tests/differential/class/clone-is-shallow-like-phps.nvst:1`. Where Novis is deliberately
      *unlike* PHP the case belongs in `tests/conformance/` instead, so this slice is only for what
      must agree.

## Backlog

- `tools/bench.py --serve-vs-fpm --record benches/serve.json` — the flag does not exist and no
  `benches/serve.json` is on disk; the check is `docs/agent/loop-goal.toml:4067` and M7's *Verify*
  paragraph (`docs/plan/m7.md`) owns what it must record. Needs PHP-FPM and a load generator on this
  machine, which is a bigger question than a slice.
- `docs/agent/goals/6-server.toml` is kept byte-identical to `docs/agent/loop-goal.toml` by hand; a
  session that edits one must edit both, since the former is what a re-install copies back.
