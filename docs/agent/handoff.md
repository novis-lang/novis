# Handoff

## State

Goal `lang-attributes`, feature 1 of 3 done: `an-attribute-is-a-shape-literal-attached-to-a-declaration`
is `complete.` in `python tools/dossier.py --id`. Nothing is blocked.

The two features left in this goal are different in kind from the first, and that is the decision the
next session opens with. Both are **compiler tables with no run-time surface**: `#[Core\Api]` feeds
`nvs build --openapi`, and `#[Core\Command]`/`#[Core\Option]` feed a table nothing in this build
dispatches to. A `docs/examples/` program prints its own stdout and a bench needs a loop, so neither
proof can show the document or the dispatch. What a program *can* do is declare the attributes and read
the table back, and what the document itself is owed is a conformance case over the CLI output.

## Next group

Both remaining features of this goal — one file set: `docs/reference/lang/90-attributes.md`,
`docs/examples/lang/attributes/`, `tests/hostile/lang/attributes/`, `benches/members/lang/attributes/`,
`tests/conformance/core/`.

- [ ] **`lang:attributes/core-api-and-the-openapi-document`** — owes about, examples, hostile, perf,
      tests. The reference's own worked example and its expected JSON are at
      `docs/reference/lang/90-attributes.md:382`; `tests/conformance/core/a-route-table-answers-a-url-for-every-route-it-holds.nvst`
      is the nearest landed case over a route table. `rule:attributes/api-adds-and-cannot-contradict`
      is what the payload check owes a proof of.
- [ ] **`lang:attributes/core-command-and-core-option-the-command-table`** — owes about, examples,
      hostile, perf, tests. `docs/reference/lang/90-attributes.md:435`, and
      `tests/conformance/core/a-command-table-answers-its-own-help.nvst` is the landed case to credit
      or extend with a `covers:` marker rather than duplicate.

## Backlog

- The debug binary overflows its stack a level past the parser's recursion guard, where release is
  clean at every depth. Recorded as a playbook trap rather than a crate `# Known gaps` item: the
  shipped build is correct, so `owners.py --check` has no milestone to scope it to. The work left is
  a parse running on a stack it sizes itself, which touches every entry point.
- An array literal inside an attribute payload types as `array<mixed>`, so
  `get<{rows: array<int>}>` answers `null` for `#[{rows: [1, 2, 3]}]`. This is **not** an attributes
  bug: `Rows $x = {rows: [1, 2, 3]};` is refused the same way, and `E0414` says `var` cannot infer an
  array literal's element type. Do not re-probe it.
- A class constant is an attach site in `docs/reference/lang/90-attributes.md:71` and not in
  `rule:attributes/attach-sites-and-forms`, which names four sites. Retrieval takes a property or a
  parameter as its second argument, so a constant's payload has no spelling that reads it back.
