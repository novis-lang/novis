# Handoff

## State

**Goal 26, stage 3's reader half is on disk and green.** `Core\Xml::reader` answers a
`Core\Xml\Reader`, whose two members are `read` (the next node, or `null`) and `depth` (how deeply
nested that node was). A reader's whole state is its own slots — the document's text, a cursor, the
stack of open names, two counts and a flag — so it holds no native allocation and what a walk
*builds* is one node. Three of stage 3's five acceptance tests pass: the two reader pins and
`no_operation_is_available_through_both_the_tree_and_the_stream`, which reads the stream half off
the registry so a class registered under `Core\Xml\` joins the sweep by existing.

**Two decisions are recorded in `crates/nvs-stdlib/src/xml.rs`'s module doc**, per the goal's
standing decision that tree ergonomics resolve toward the narrower surface: a walk never answers a
`Document` node, and a closing tag is not a node, which is why `depth` exists at all.

**`rule:core-classes/xml-tree-and-stream` stays `designed` on purpose** — the writer is not on disk,
and the rule states both shapes. The wrap that lands the writer flips it to `shipped` and re-renders.

**`examples/html-sanitize.nvs` is still the failing acceptance check** and is still not a regression:
it cannot be written before `Core\Html::sanitize` exists in stage 5.

## Next group

**Stage 3: the writer** — one file set, `crates/nvs-stdlib/src/xml.rs` plus its registration in
`crates/nvs-stdlib/src/registry.rs` and new cases under `tests/conformance/core/`. The whole group
is specified by `rule:core-classes/xml-tree-and-stream`.

- [ ] **The writer**, a `Core\Xml\Writer` whose own state enforces nesting, so a caller cannot emit
      ill-formed output: what is open is the writer's, not an argument. Its entry row joins the
      class block at `crates/nvs-stdlib/src/xml.rs:138` beside `reader`, its class goes beside
      `READER` at `crates/nvs-stdlib/src/xml.rs:383`, its symbols the `address` arm at
      `crates/nvs-stdlib/src/xml.rs:475`, and its registration the list at
      `crates/nvs-stdlib/src/registry.rs:1707`. Pins
      `the_writer_enforces_nesting_from_its_own_state_and_not_from_the_caller` and
      `an_unclosed_element_at_the_end_is_an_error_and_not_a_document` — the second is the writer's
      half; the reader's is already pinned by
      `tests/conformance/core/xml-a-reader-refuses-where-the-walk-reaches-it-and-does-not-advance-past-it.nvst`.
      Note that no member anywhere may take a `Core\Xml\Node`, which
      `a_parsed_tree_has_no_path_back_into_execution` at
      `crates/nvs-stdlib/src/xml.rs:1932` enforces — so the writer is written to, never handed a
      tree.
- [ ] **Three `.nvst` cases per new member**, the floor
      `crates/nvs-stdlib/tests/conformance_coverage.rs:156` asserts, each asking a different
      question. `Core\Xml\Writer` is reached by the same `builds` attribution the reader is, so one
      case naming `Core\Xml::writer` credits every member it then calls with an arrow.
- [ ] **Re-run the disjointness sweep against the new rows** at
      `crates/nvs-stdlib/src/xml.rs:2148`: it refuses a stream member that spells one of the node
      family's, so a writer wanting `text` or `name` has to be renamed rather than exempted.
- [ ] **Close the rule and the gap**: `rule:core-classes/xml-tree-and-stream` to `shipped` in
      `docs/rules/core-classes.json` with `python tools/rules.py --render`, and known gap 1 in
      `crates/nvs-stdlib/src/xml.rs:90` deleted, since it names the writer and nothing else.

## Backlog

- Stage 4, the WHATWG parser on `Core\Html` — `rule:core-classes/html-parsing`.
- Stage 5, `Core\Html::sanitize` and `examples/html-sanitize.nvs` — the failing acceptance check.
- Nothing serialises a tree back out: `crates/nvs-stdlib/src/xml.rs`'s known gap 2.
- A name is the name as written, prefix and all: that module's known gap 3.
