# Handoff

## State

**Goal 26, stage 2 is closed.** Every name in the stage-2 acceptance check exists and is green in
`crates/nvs-stdlib/src/xml.rs`'s test module: the node family, the three refusals, the taint sweep and
the inertness proof — six `#[test]`s over the parser and the registry rows.

**The DOCTYPE question is decided, toward the whole refusal.** ADR 0168 records it and creates two
rules: `rule:core-classes/xml-tree-and-stream` (`designed` — its central claim is not assertable until
the stream exists, and stage 3 flips it) and `rule:core-classes/xml-refuses-by-construction` (`shipped`).
The check that presumed goal 25's ratio-and-ceiling is renamed
`a_billion_laughs_document_is_refused_at_its_doctype_rather_than_bounded` in both goal tomls, and the
goal prose in both `.md` copies now says the same thing.

**Stage 0 has not landed.** `crates/nvs-stdlib/src/html.rs`'s *Known gaps* still says the tree waits on
existing at all, and `rule:core-classes/html-parsing`'s *Not shipped* paragraph still reads as
unscheduled. Both are wrong now, and both are one edit.

**The driver's acceptance check has failed on `examples/xml-tree.nvs` being absent since session 0006.**
It is a `files` fixture, not a test — the goal's fixture list names it and nothing has written it.

**The goal's one pre-authorized ADR number is now spent** (0168); next free is 0169, and this goal may
not open it.

## Next group

**Stage 0: the catch-up, plus the fixture stage 2 still owes** — one file set, all of it prose or a
program over work that is already on disk and green.

- [ ] **Write `examples/xml-tree.nvs`**, the fixture the goal's file list names at
      `docs/agent/goals/26-xml-tree.toml:9` and the earliest-stage acceptance failure. A program that
      parses a document, walks the five kinds and shows a refusal —
      `rule:core-classes/xml-tree-and-stream` for the shape it demonstrates and
      `rule:core-classes/xml-refuses-by-construction` for the refusal. Run it with
      `target/debug/nvs.exe run examples/xml-tree.nvs`; the expected output is frozen data, so check
      what the goal's fixture block wants before freezing anything.
- [ ] **Strike known gap 1 in `crates/nvs-stdlib/src/html.rs:26`**, which says the tree waits on
      existing at all. It exists: `crates/nvs-stdlib/src/xml.rs:173` is the node family and
      `rule:core-classes/html-parsing` is what both parsers share. Rewrite the gap as what `Core\Html`
      still owes — the WHATWG parse itself — rather than deleting it.
- [ ] **Amend `rule:core-classes/html-parsing`'s *Not shipped* paragraph** at
      `docs/rules/core-classes/html-parsing.md:18`, which reads as unscheduled. Goal 26 is the milestone
      that schedules it, and half of it has landed, so the paragraph states what is on disk and what
      stage 4 still owes. A fragment edit needs `python tools/rules.py --render`, which the wrap runs.

## Backlog

- Stage 3, the stream — the reader and writer, and the flip of `rule:core-classes/xml-tree-and-stream`
  to `shipped` with `no_operation_is_available_through_both_the_tree_and_the_stream` as its guard
  (`docs/agent/goals/26-xml-tree.toml:6652`).
- Stage 4, the WHATWG parser on `Core\Html` over `html5ever` (`rule:core-classes/html-parsing`).
- Stage 5, `Core\Html::sanitize` as a launderer (`rule:core-classes/html-sanitize`, still `designed`).
- `Core\Xml`'s serialization: known gap 2 in `crates/nvs-stdlib/src/xml.rs`, which stage 5's round trip
  needs.
- Namespace resolution — known gap 3 in the same module doc; no goal owns it.
- `§16 Core\Metrics` and `§16 Core\Signature` are all that is left in
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
