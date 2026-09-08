# Handoff

## State

**Goal 19 — the binding site has landed; the runnable fixture is what the goal still owes.** A route
capture declared at a class implementing `Parses` other than `Core\Uuid` now matches on shape, crosses
carrying its class name, and converts where the match reaches the program: a segment the class refuses
throws there, over a route that did match, which a handler answers `400`. ADR 0160 and the two rule
edits it names landed in the same commit.

**The crossing is `crates/nvs-stdlib/src/router.rs`'s `capture_value` and `parsed`.** Both take a
`&mut Ctx` now, so `match_value` does too, and `Core\Request::route()` takes its match off the carrier
by clone rather than by borrow — the doc comment on that member says what it spends.
`Core\Router\Match::params` answers `array<tainted string|int|uint|decimal|Core\Uuid|Parses>`; the last
member is an interface, admitted by a new `GLOBAL_INTERFACES` roster beside `EXCEPTION_TREE` in
`crates/nvs-stdlib/src/registry.rs`'s gate.

**Two things the goal names are still open, and neither is the binding site.** The `#[Query]` half of
`rule:routing/a-bad-query-value-is-a-400` binds nothing at all — the fixture's `tag is absent` and
`tag is release-notes` lines cannot come from a bound `#[Query]` parameter today. And stage 4's own
`cargo-named` checks name five `-p nvs-runtime` tests that do not exist, one of which
(`a_segment_parse_refuses_is_no_match_rather_than_a_matched_bad_value`) states the answer this goal's
§ *Standing decisions* settled the other way; it is a check name to amend, not work to do.

## Next group

**Stage 5: the runnable fixture and what the acceptance check needs to run it** — one file set:
`examples/parses.nvs`, `docs/agent/loop-goal.toml`, `crates/nvs-stdlib/src/router.rs`.

- [ ] **`examples/parses.nvs`, and the leg argument that gives it a request** — the check at
      `docs/agent/loop-goal.toml:5921` carries no `args`, so `tools/loop.py`'s `program_check` runs
      `nvs run examples/parses.nvs` with no request at all and `Core\Request::route()` refuses through
      `crates/nvs-stdlib/src/request.rs:1690`'s `inbound_of`. Either add
      `args = ["--request", "examples/parses.nvsr"]` beside the `want` list its comment already
      describes, or write the fixture against `Core\Router::match` at
      `crates/nvs-stdlib/src/router.rs:913`, which takes the same walk and needs no request. The two
      `tag is …` lines need a decision either way: nothing binds a `#[Query]` parameter yet, so they
      can only come from `Core\Request::query()` read by hand.
- [ ] **The three conformance cases stage 4 names** — `a-class-that-parses-stands-where-a-uuid-stands`,
      `a-segment-the-class-refuses-never-reaches-the-handler` and
      `a-class-claiming-the-contract-owes-its-one-member`, under `tests/conformance/core/`. The shapes
      are `crates/nvs-stdlib/src/router.rs:1007`'s arm read three ways; `rule:expressions/try-parse`
      is what the third pins.

## Backlog

- Stage 4's five `-p nvs-runtime` check names have no tests and one contradicts the goal's standing
  decision — `docs/agent/loop-goal.toml:5893`.
- `#[Query]` binding converts nothing — `rule:routing/a-bad-query-value-is-a-400`.
- The `Parses` roster entries in `docs/novis.md`'s two prose lists — `docs/agent/loop-goal.md` stage 5.
- The diagnostic corpus stage 5 names — `docs/agent/loop-goal.md` stage 5 item 3.
