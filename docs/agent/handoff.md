# Handoff

## State

**Goal 26, stage 2. The tree exists.** `crates/nvs-stdlib/src/xml.rs` is new and registered:
`Core\Xml::parse`, `Core\Xml\Node`'s five instance members (`kind`, `name`, `text`, `attributes`,
`children`) over five slots, and `Core\Xml\NodeKind`'s five cases, over a hand-written strict XML
reader in the same file. Five `.nvst` cases under `tests/conformance/core/xml-*`; the unit test
`element_text_comment_processing_instruction_and_document_are_the_whole_family` is green.

**Stage 0 has not landed** — `crates/nvs-stdlib/src/html.rs`'s *Known gaps* (`html.rs:26`) still says
the tree waits on existing at all, and `rule:core-classes/html-parsing`'s *Not shipped* paragraph
(`docs/rules/core-classes/html-parsing.md:18`) still reads as unscheduled. Both are now wrong.

**No ADR was opened.** The node family was already decided by `rule:core-classes/html-parsing`, so
this slice implements a rule rather than making one. The goal's one pre-authorized ADR number (next
free is 0168) is unspent and belongs with the tree/stream split and the three refusals.

**One design call, made toward the safe option and not yet recorded in a rule:** a `<!DOCTYPE …>` is
refused *whole* (`crates/nvs-stdlib/src/xml.rs:462`), so an internal subset defines nothing and there
is no expansion to bound. Stage 2 item 4's check name presumes goal 25's ratio-and-ceiling instead —
that is the first thing the next group has to settle.

**Stage 5 item 4 is already done**: registering a class and striking its line in
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` are one slice by that file's own rule,
so `§17 Core\Xml` is gone and only `§16 Core\Metrics` and `§16 Core\Signature` remain.

## Next group

**Stage 2: the refusals, the taint and the record that decides them** — one file set:
`crates/nvs-stdlib/src/xml.rs`, a new `docs/decisions/0168.md`, `docs/rules/core-classes/`.

- [ ] **Decide the DOCTYPE question and record it** — ADR 0168 with a rule fragment under
      `docs/rules/core-classes/`, covering the tree/stream split and the three refusals
      (`rule:core-classes/html-parsing` for the family it does not reopen). The whole-refusal position
      is at `crates/nvs-stdlib/src/xml.rs:462`; the alternative it beat is an internal subset expanded
      under goal 25's ceiling. Then fix the check name in `docs/agent/loop-goal.toml` and
      `docs/agent/goals/26-xml-tree.toml` if the decision keeps the refusal.
- [ ] **Item 4's three tests** — `an_external_entity_is_not_a_code_path_rather_than_a_flag_defaulting_to_off`,
      `a_dtd_naming_an_external_subset_is_refused_rather_than_fetched` and
      `a_billion_laughs_expansion_is_bounded_by_the_ceiling_goal_25_landed`, in
      `crates/nvs-stdlib/src/xml.rs:1102`'s test module. The parser already refuses all three; what is
      owed is the assertion. `Reader::reference` at `crates/nvs-stdlib/src/xml.rs:601` is the whole of
      what this module resolves, which is what the first name is a claim about.
- [ ] **Item 5 — `every_string_read_out_of_a_parsed_tree_is_tainted`**, same test module. The rows are
      `CoreTy::TaintedStr` at `crates/nvs-stdlib/src/xml.rs:173`; a test asserts the *rows*, since the
      qualifier is erased before codegen and no run-time value carries it.
- [ ] **Item 3 — `a_parsed_tree_has_no_path_back_into_execution`**, same test module, on
      `rule:tooling/reflection-and-source-parsing-are-core-features`'s reading for the AST: the roster
      at `crates/nvs-stdlib/src/xml.rs:173` is five slot reads and nothing that evaluates.

## Backlog

- Stage 0's two prose items: `crates/nvs-stdlib/src/html.rs:26` and `docs/rules/core-classes/html-parsing.md:18`.
- Stage 3's stream (reader and writer), which known gap 1 in `crates/nvs-stdlib/src/xml.rs` names.
- Nothing serialises a tree back out — known gap 2, and stage 5's `sanitize` needs it.
- Namespace prefixes are not resolved — known gap 3.
- `[context] modules` gained `nvs-stdlib/src/xml.rs` by the driver's sweep; `[context] shapes` wanted
  nothing this session did not have.
