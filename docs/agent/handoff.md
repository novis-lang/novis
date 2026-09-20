# Handoff

## State

Goal `core-ast-and-2-more` (dossier) is met — all 9 items landed. Every feature in `Core\Ast`,
`Core\Ast\Node` and `Core\Attributes` reports `complete` from `python tools/dossier.py --id
'<feature>'`, and the acceptance check that was red,
`python tools/dossier.py --verify --group 'Core\Attributes'`, now reports `nothing owed` with 6
examples and 2 attacks green. `owners.py --closes` and `playbook.py --closes` name nothing for this
goal, and `verify.py --doc` is green.

One finding from the earlier group is still recorded rather than fixed, as
`rule:testing/a-failing-proof-is-fixed-or-recorded` allows: `Core\Ast::parse` spends time in the
square of a line's length. It is `# Known gaps` item 1 in `crates/nvs-stdlib/src/ast.rs`, owned by
M12, with the marked attack at
`tests/hostile/core/Ast/parse/02-a-program-written-on-one-long-line.nvs`.

## Next group

The goal is met, so the driver's goal switch replaces this file with
`docs/agent/goals/dossier/95-core-bigint-1-2.handoff.md`. Its opening group, one file set —
`crates/nvs-stdlib/src/bigint.rs`, `docs/examples/core/BigInt/`, `tests/hostile/core/BigInt/` and
`benches/members/core/BigInt/`, under `rule:testing/feature-proofs`:

- [ ] **`Core\BigInt::abs`** — owes `about.md`, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:248`
- [ ] **`Core\BigInt::add`** — owes `about.md`, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:149`
- [ ] **`Core\BigInt::compareTo`** — owes `about.md`, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/bigint.rs:284`

## Backlog

- `Core\Attributes`' two members were re-attributed rather than given new cases: the Novis side is
  `tests/conformance/lang/attributes-retrieval-answers-by-shape-in-declaration-order.nvst` and
  `tests/conformance/core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst`, both of which
  already asserted the member (`rule:testing/proof-attribution`).
- `all`'s bench records 8 allocations per read of a 3-entry roster where 4 was the prediction; the
  extra is array growth and the index copy, not the retrieval. `benches/members/README.md`
  § *What the numbers mean* is where a profiler pass would start.
