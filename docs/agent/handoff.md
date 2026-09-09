# Handoff

## State

**Goal `xml-tree`, stage 3 is complete and green.** `Core\Xml::writer({indent?: string})` answers a
`Core\Xml\Writer` with ten members — the two pairs (`startDocument`/`endDocument`,
`startElement`/`endElement`) and the six single calls (`content`, `attribute`, `comment`, `cdata`,
`instruction`, `doctype`) that spec § 17 keeps out of PHP's doubled roster. Its whole state is its own
slots, so what is open is never an argument: `endElement` takes no name, `endDocument` refuses a tree
rather than emitting one, and every refusal writes nothing. All five of stage 3's acceptance tests
pass.

**`rule:core-classes/xml-tree-and-stream` is `shipped`** and names its four guards; its last paragraph
now says what each half of the stream spends, since a writer holds the document it is building.

**One asymmetry is deliberate and stated in three places** — `doctype`'s reference card, the module
doc and
`tests/conformance/core/xml-a-writer-answers-a-document-its-own-reader-reads-back.nvst`: the writer
writes a `<!DOCTYPE …>` and `Core\Xml::parse` refuses one whole, so that is the one construct this
class writes and will not read back. Nobody has decided whether a subset-free declaration should
parse; `rule:core-classes/xml-refuses-by-construction` currently says no.

**`examples/html-sanitize.nvs` is still the failing acceptance check** and is still not a regression:
it cannot be written before `Core\Html::sanitize` exists in stage 5.

## Next group

**Stage 4: the WHATWG parser, on `Core\Html`** — one file set, `crates/nvs-stdlib/src/html.rs` plus
`crates/nvs-stdlib/Cargo.toml`, the family in `crates/nvs-stdlib/src/xml.rs` and new cases under
`tests/conformance/core/`. The whole group is specified by `rule:core-classes/html-parsing`.

- [ ] **`html5ever` as a dependency of `nvs-stdlib`, driven through a Novis-owned tree builder** into
      `crate::xml`'s own `Parsed` nodes, so the boundary between the crate and the tree is where
      stage 2's family is enforced. The dependency goes in `crates/nvs-stdlib/Cargo.toml:11`, and
      `rule:packaging/a-c-dependency-answers-two-questions` is the audit it has to pass — it is Rust,
      so it passes on the first question, and the record should say so rather than leave it unasked.
      Specified by `rule:core-classes/html-parsing`.
- [ ] **`Core\Html::parse(string $document): Core\Xml\Node`**, the entry that never fails. Its row
      joins the class block at `crates/nvs-stdlib/src/html.rs:107`, its symbol the `address` arm at
      `crates/nvs-stdlib/src/html.rs:248`, and it answers `CoreTy::Instance(NODE_NAME)` from
      `crates/nvs-stdlib/src/xml.rs:123`. Pin
      `tag_soup_produces_a_document_because_the_parser_has_no_failure_mode` and
      `the_parser_produces_core_xmls_own_node_family` — the second is best asked as an *agreement*
      over `crate::xml::KIND`'s five cases rather than over one document.
- [ ] **`it_is_an_entry_on_core_html_and_there_is_no_html_mode_on_the_xml_parser`**, read off the
      registry the way `no_operation_is_available_through_both_the_tree_and_the_stream` at
      `crates/nvs-stdlib/src/xml.rs:3150` reads the stream half: no row on `Core\Xml` may take a
      parameter that selects an algorithm, and no row on `Core\Html` may answer a reader or a writer.
- [ ] **Three `.nvst` cases per new member**, the floor
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155` holds. One case may ask the whole roster,
      so three cases asking three different questions close the whole class at once — that is how
      stage 3's three writer cases were written.

## Backlog

- Nothing serialises a tree — `crates/nvs-stdlib/src/xml.rs` § *Known gaps* 1, and stage 5's
  `sanitize` needs it.
- A qualified name is the document's own spelling, with no `xmlns` resolution — same section, gap 2.
- Indenting puts whitespace text nodes between elements, which a parse answers as text — same
  section, gap 3, and the writer's `indent` option is where a program meets it.
- Whether a subset-free `<!DOCTYPE …>` should parse — `rule:core-classes/xml-refuses-by-construction`
  says no, and the writer can now write one.
- `examples/html-sanitize.nvs`, blocked on stage 5 — `docs/agent/loop-goal.md`.
