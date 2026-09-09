# Handoff

## State

**Goal 26, stages 0 and 2 are closed.** The tree half is on disk and green in
`crates/nvs-stdlib/src/xml.rs` — `Core\Xml::parse`, the five-case node family, the refusals and the
taint sweep — and stage 0's two catch-up edits have landed: `crates/nvs-stdlib/src/html.rs`'s *Known
gaps* and `rule:core-classes/html-parsing`'s closing paragraph both say what is true now, that the
tree exists and only the WHATWG builder over it is missing.

**`examples/xml-tree.nvs` is written and runs**, so the acceptance check that has failed since session
0006 is closed. It parses one document holding every kind, walks it with an exhaustive `match` over
`Core\Xml\NodeKind` (no `default` — the enum is closed and the compiler accepts that), and catches
both a malformed document and a `<!DOCTYPE …>`.

**The next `files` fixture to fail is `examples/html-sanitize.nvs`, and that is not a bug.** It cannot
be written before `Core\Html::sanitize` exists in stage 5; it is an item still open, not a regression.

**Stage 3 is untouched** — nothing in the tree holds a reader or a writer, and
`rule:core-classes/xml-tree-and-stream` stays `designed` until one does.

**The goal's one pre-authorized ADR number is spent** (0168); next free is 0169, and this goal may not
open it.

## Next group

**Stage 3: the stream** — one file set, `crates/nvs-stdlib/src/xml.rs` plus its registration in
`crates/nvs-stdlib/src/registry.rs` and new cases under `tests/conformance/core/`. Every slice below
is specified by `rule:core-classes/xml-tree-and-stream`; the acceptance names are the five in the
`stage = "3 stream"` check.

- [ ] **The reader**, a `Core\Xml\Reader` class answering stage 2's node family one node at a time and
      holding one window rather than the document. Its rows join the class block at
      `crates/nvs-stdlib/src/xml.rs:112`, its symbols the `address` arm at
      `crates/nvs-stdlib/src/xml.rs:340`, and its registration the list at
      `crates/nvs-stdlib/src/registry.rs:1697`. Pins
      `the_reader_answers_stage_twos_node_family_one_node_at_a_time` and
      `the_reader_holds_one_window_rather_than_the_document`.
- [ ] **The writer**, whose own state enforces nesting so a caller cannot emit ill-formed output, and
      for which an unclosed element at the end is an error rather than a document. Same three anchors;
      it does not reuse `NODE` at `crates/nvs-stdlib/src/xml.rs:173`, because a writer is given
      elements rather than handed nodes. Pins
      `the_writer_enforces_nesting_from_its_own_state_and_not_from_the_caller` and
      `an_unclosed_element_at_the_end_is_an_error_and_not_a_document`.
- [ ] **The two shapes share no operation**, which is the claim
      `rule:core-classes/xml-tree-and-stream` is currently `designed` for: assert it over the registry
      at `crates/nvs-stdlib/src/xml.rs:112` as
      `no_operation_is_available_through_both_the_tree_and_the_stream`, then flip the rule's `status`
      in `docs/rules/core-classes.json` and re-render with `python tools/rules.py --render`.
- [ ] **Three `.nvst` cases per new member**, the floor asserted at
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155`; the five existing
      `tests/conformance/core/xml-*.nvst` are the spelling to copy, and
      `tests/conformance/core/xml-parse-answers-one-node-family-through-the-tree.nvst:29` is the walk
      a reader case is written against.

## Backlog

- `examples/html-sanitize.nvs` — the last missing `files` fixture, blocked on stage 5's `sanitize`.
- Stage 4, the WHATWG parser on `Core\Html` — `crates/nvs-stdlib/src/html.rs`, `rule:core-classes/html-parsing`.
- Stage 5, `Core\Html::sanitize` answering a `Markup` by rebuilding — `rule:core-classes/html-sanitize`.
- `crates/nvs-stdlib/src/xml.rs`'s known gap 3, namespace prefixes unresolved — that module's doc.
