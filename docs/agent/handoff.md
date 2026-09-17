# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way, and stage 4's first acceptance
check is green.** `Core\Xml\Node::namespaceUri` answers the URI the nearest enclosing `xmlns`
declaration bound an element's prefix to, `name` still answers the spelling the document wrote, and
the tree door and the streaming reader resolve alike. `crates/nvs-stdlib/src/xml.rs`'s § *A name is
as written* is the home of what that spends.

**Its one deviation from the sheet, for the record.** The sheet priced xml gap 1 as a lookup that
"costs a walk up the tree", with no per-node cost. A node holds its children and no parent — and a
parent link would be a reference cycle in a refcounted tree — so an element cannot walk anywhere at
run time. Taking the document as a parameter is refused outright by
`a_parsed_tree_has_no_path_back_into_execution` (see the playbook). So the member is the decided one
and the resolution happens while each door descends, carried on a sixth slot: one slot per node and
one string per element in a namespace.

`json.rs` has no known gaps left either — its decided widening landed a day earlier under another
goal (`nvs_runtime::CodecField::default`, and the `toJson` lookup ahead of the derived list), so the
item was struck rather than built and the rule fragment already reads as the code does. `python
tools/owners.py --closes decided-closures` names 21 gaps now, down from 23.

## Next group

**Stage 4: the regex class's two closeable gaps** — one file set: `crates/nvs-stdlib/src/regex.rs`,
`crates/nvs-config/src/tree.rs` and `crates/nvs-types/src/core_lib.rs`. The first two together are
stage 4's third check's first test, `the_regex_step_budget_is_a_limits_directive_with_the_constant_as_its_default`.

- [ ] **`crates/nvs-config/src/tree.rs:171` — `[limits]` gains the regex step budget.** An ordinary
      `Runtime`-class key (`rule:config/three-changeability-classes`), whose default is the constant
      `crates/nvs-stdlib/src/regex.rs:807` states today, so a request may widen or narrow it and a
      host may state it once.
- [ ] **`crates/nvs-stdlib/src/regex.rs:807` — the budget is read from that directive.** Gap 2's
      numbered item goes, and `rule:core-classes/regex-two-tiers`'s closing paragraph — which says
      the default is a stated constant "because there is no configuration subsystem in front of it
      yet" — is amended in the same slice, per the goal's standing decisions.
- [ ] **`crates/nvs-types/src/core_lib.rs:377` — `qual_of` reads a parameter's declared `Qual`
      whatever its type.** regex.rs gap 1: a `Pattern|string` union refuses a tainted argument by
      the default rather than by a mark a reader can find (`rule:security/regex-pattern-is-a-sink`),
      and a union has nowhere to hold `Qual::Launder`.

## Backlog

- The prepared-pattern channel — `crates/nvs-stdlib/src/cldr.rs:212` and `time.rs:102`, stage 4's
  third check's second test, and the goal's one ADR slot (next free 0192).
- `crates/nvs-stdlib/src/regex.rs:89` gap 3, the compiled-pattern cache's accounting bracket — M6's,
  per the goal sheet.
- The other 18 gaps `python tools/owners.py --closes decided-closures` names, across
  `nvs-diagnostics`, `nvs-stdlib`, `nvs-syntax` and `nvs-types`.
