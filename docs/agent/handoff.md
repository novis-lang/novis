# Handoff

## State

Goal `lang-attributes`, **7 of the group's 8** features complete in `python tools/dossier.py --gate
--group lang:attributes`. This session landed `the-names-the-compiler-acts-on`,
`core-route-and-core-access-the-route-table` and
`core-program-implementing-i-every-class-implementing-an-interface`. One feature is left, and the
goal's stage-2 check goes green the session it lands. Nothing is blocked.

Two things the landed proofs settled. **A recognized name stays in the structural roster**:
`Core\Attributes::all<{}>` at a method carrying `#[Core\Command]` answers one entry, so recognition
and retrieval are independent, and the `[skip]` note about `#[Core\Api]` answering `null` is about
that attribute rather than about recognized names in general. And **a feature whose whole surface is
compile-time takes a `[skip]` perf entry with its reason** — `the-names-the-compiler-acts-on` has one
in `tools/data/dossier-policy.toml`, because a loop there would re-measure the codec, route row or
command row the recognized attribute built, each of which carries its own figure.

A perf figure in this group goes stale when `docs/reference/lang/90-attributes.md` is edited, since
that chapter is what every `lang:attributes/…` feature is implemented at, so a slice that corrects the
chapter re-measures with `--record-perf --only` before it wraps.

## Next group

**Stage 2: the dossier, the last feature of the group and then the goal's own end** — one file set:
`docs/reference/lang/90-attributes.md`, `docs/examples/lang/attributes/`,
`tests/hostile/lang/attributes/`, `benches/members/lang/attributes/`, `tests/conformance/lang/`,
`tests/conformance/reject/`. The second item runs only once the first is green.

- [ ] **`lang:attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec`** — owes
      about, examples, hostile, perf, tests. The chapter section is
      `docs/reference/lang/90-attributes.md:129`, the two recognized names and what each may carry
      are `crates/nvs-types/src/derive.rs:74`, and the codec the attribute generates is
      `crates/nvs-types/src/derive.rs:1526`. `rule:core-classes/derive-attribute` is the opt-in and
      `rule:core-classes/derive-field-list` the per-field override; a class carrying neither the
      attribute nor a hand-written `fromJson` is refused at a document door with `E0821`, and
      `Core\Json::encode` of an instance with no codec throws a `LogicError` at run time, which is
      what a conformance case can catch.
- [ ] **Reach the goal**, whose stage-2 check is `docs/agent/loop-goal.toml:12386` and passes the
      moment the item above is green. Run `python tools/verify.py --doc` and fix every
      broken link it names, then `python tools/owners.py --closes lang-attributes` and `python
      tools/playbook.py --closes lang-attributes`, closing or re-ownering every gap they name, and
      write `DONE` to `.loop/status.txt`. Those are the gates a goal meets only at its end.

## Backlog

- `Core\Router::url` costs 14 allocations and 607 bytes per link (`docs/perf/members.ndjson`), which
  is a figure to improve rather than a failure; the ledger now has the baseline.
- A `{path...}` value keeps its `/`s, so `Core\Router::url("tree", ["path" => "../../etc/passwd"])`
  answers `/files/../../etc/passwd`, which normalizes out of its own route; pinned as current
  behaviour in `tests/conformance/lang/routes-the-compiled-table-answers-a-link-of-every-capture-kind.nvst`
  and worth an ADR question when a server exists to dispatch it.
- `docs/reference/lang/90-attributes.md:124`'s `<!-- generated: attributes -->` table renders
  nothing in the chapter as it stands on disk; the roster it would list is
  `crates/nvs-types/src/derive.rs:68`.
