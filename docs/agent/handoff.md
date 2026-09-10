# Handoff

## State

**Goal `xml-tree` is met** — the driver's own acceptance sweep after session 0003 passed whole, 449
checks. This session took the follow-ups the goal did not owe, so nothing here is a gate.

**`Core\Xml\Node::source(): tainted string` ships** — the tree half's way back out, so a walked node
is written as document text without replaying it into a `Core\Xml\Writer` a call at a time. It sits
on the node because a member *taking* a tree is the path back into execution that
`a_parsed_tree_has_no_path_back_into_execution` closes, and what it answers is text. Its contract's
home is `crates/nvs-stdlib/src/xml.rs`'s module doc § *the way back out of a tree*: XML rules
whichever door parsed the tree, an end tag on every element, and a refusal where
`Core\Html::parse` recovered. The walk is iterative over a heap stack for `instance_of`'s reason.
Known gap 1 — *nothing serialises a tree* — is gone and the other two are renumbered.

**Five conformance cases landed**: the round trip and its fixed point, the refusals a WHATWG-parsed
tree reaches, a subtree with its text and attribute values escaped, the depth bound asserted on both
sides, and one agreement case putting a sanitized document through both doors and comparing what
each writes back.

No rule fragment changed. The member lands under `rule:core-classes/html-parsing`'s *serialization
follows the door* clause and `rule:core-classes/xml-tree-and-stream`, both of which already say what
it does; `docs/rules/` is untouched.

## Next group

**Follow-ups this goal did not owe** — one file set, `crates/nvs-stdlib/src/html.rs` and
`docs/rules/core-classes.json`. A goal switch discards this list, which is the right outcome if the
sweep agrees the goal is met.

- [ ] **`class` on the sanitizer's allowlist, or a written reason it is not there.** The standing
      decision in `docs/agent/loop-goal.md` § *Standing decisions* says the list gains an element in
      a commit with a reason rather than by a parameter, and `rule:core-classes/html-sanitize` is
      what specifies the closed list; `class` is the first attribute a real application will ask
      for, and the table is `crates/nvs-stdlib/src/html.rs:1273`.
- [ ] **The serialiser's cases named in the rule they guard.** `rule:core-classes/xml-tree-and-stream`
      lists what fails when it is broken and does not yet name the four cases this session wrote;
      the entry is `docs/rules/core-classes.json:220`, and a session touching `docs/rules/` owes
      `python tools/rules.py --render`, which the wrap runs for it.

## Backlog

- Goal `xml-tree`'s acceptance list is green; the driver switches the chain, not a session.
- Per-crate known gaps live in each crate's module doc, `docs/agent/carried-gaps.md` for what must
  survive a goal switch.
