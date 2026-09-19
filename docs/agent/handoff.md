# Handoff

## State

Goal `types-interface` is met. All six reserved global interfaces — `Comparable`, `Iterable`,
`Iterator`, `Parses`, `PropertyObserver`, `Stringable` — now have a description, examples and a
conformance case, and `python tools/dossier.py --verify --group 'types:interface'` prints
`nothing owed`, `0 failed`, `0 failed`. An interface owes no bench and no attack, which is data in
`tools/data/dossier-policy.toml` rather than a decision this session made.

Five of the six were written by the fan-out the goal authorizes (`dossier.py --partition`), one
worker per feature; this session wrote `Comparable`, then read every page and program and ran every
new case before committing. Two workers hand-derived an `--EXPECT--` block they could not execute;
both match the binary byte for byte.

`Parses`' seeded `tryParse` default is **not** reachable through an implementor — `Slug::tryParse($s)`
is `E0309` — and that bound is now stated in the prose at `crates/nvs-hir/src/interfaces.rs:61` and
`crates/nvs-types/src/iter_lib.rs:118`, which previously promised a reader the opposite.

## Next group

**Goal `lang-programs`, stage 2 — one file set: `docs/reference/lang/10-programs.md`.** The chain
advances at the goal switch and installs that goal's own sibling handoff; these are its first three
items, and each owes all four proofs rather than the two an interface owes:

- [ ] **`lang:programs/a-program-is-a-file-of-top-level-statements`** — owes examples, hostile, perf,
      tests. `docs/reference/lang/10-programs.md:9`
- [ ] **`lang:programs/a-complete-program-annotated`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/10-programs.md:26`
- [ ] **`lang:programs/also-a-line-comment`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/10-programs.md:114`

## Backlog

- `Parses` seeds a `tryParse` default with no compiled function, so an implementor cannot reach it;
  the open question is whether that body gets compiled or the seed goes, since
  `rule:expressions/try-parse` gives a laundering `parse` no non-throwing twin at all. Analysis in
  `.loop/dossier-findings/w03-parses.md`, mechanism in `docs/agent/playbook.md:7488`.
- Three `Stringable` shapes still unasserted: a child that inherits `toString` without overriding it
  rendered at an implicit site, `Stringable` as a return type, and handing a `Stringable` to a
  parameter typed `string` — the last belongs under `tests/conformance/reject/`.
- The fan-out is the cheapest way to walk a generated dossier goal: five workers landed five
  features in one session's context, and the parent's window went to reviewing rather than writing.
