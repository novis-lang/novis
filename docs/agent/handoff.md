# Handoff

## State

**Goal `xml-tree` is met and its two follow-ups are now landed**, so nothing in the goal is open.
The driver's own acceptance sweep after the serialiser session passed whole; this session took the
two items that goal did not owe, and neither is a gate.

**`class` stays off `Core\Html::sanitize`'s allowlist, with its own written reason.**
`rule:core-classes/html-sanitize`'s closed list gains an element in a commit with a reason
(`docs/agent/loop-goal.md` § *Standing decisions*), and the reason for refusing this one is that
`class` and `id` are the two attributes a *receiving* page selects on: the value chooses which of
that page's own stylesheet rules and script handlers apply to attacker-supplied content, and its
meaning lives in a document this member never sees, so there is no value test to write. The home is
`crates/nvs-stdlib/src/html.rs:1266` — the paragraph beside `ELEMENTS` that already decided `id`,
`style`, `target` and the SVG/MathML namespaces. An application that needs the content styled
styles the element it puts the answer inside, which is markup it wrote itself.
`GLOBAL`'s three (`dir`, `lang`, `title`) are unchanged and no rule fragment changed.

**Six cases are named by the rules they guard**, in `docs/rules/core-classes.json`: the five the
serialiser session wrote plus this session's, each added to the `guardedBy` of the rules its own
`--TEST--` line names. `security/tainted-sources` was deliberately not given
`xml-a-node-writes-its-own-subtree…` — that case holds the serialiser's escaping, and the taint
token in its title is context rather than the rule it would break. `rules.py --render` changes
nothing generated: `guardedBy` is not rendered into the chapter.

## Next group

**`Core\Xml`'s two known gaps, each pinned by the case it has never had** — one file set:
`crates/nvs-stdlib/src/xml.rs` and `tests/conformance/core/`. Both are documented *positions*
rather than missing work, so each slice is one `.nvst` and a `guardedBy` entry, and neither adds
surface. `python tools/try.py <case>` runs one in a call.

- [ ] **A prefixed name is answered as written, and no `xmlns` is resolved.** Gap 1 at
      `crates/nvs-stdlib/src/xml.rs:122` says `<x:a/>` answers `x:a`; nothing asserts it, and
      `tests/conformance/core/xml-an-element-answers-its-attributes-in-written-order.nvst` is the
      only case that mentions `xmlns` at all. The *Edges* shape: the declaration is carried as an
      ordinary attribute and the name is the document's own spelling.
      `rule:core-classes/xml-tree-and-stream`.
- [ ] **Whitespace between elements is a text node.** Gap 2 at
      `crates/nvs-stdlib/src/xml.rs:127`: a pretty-printed document has a text node between every
      pair of siblings, and dropping them would be a guess about which whitespace mattered
      (`rule:errors/ambiguous-input-refused`). Count the children of a formatted element rather
      than reading one off — the *invariance over a sweep* shape — and pair it with
      `Core\Xml\Node::source` writing the document back unchanged.
      `rule:core-classes/xml-tree-and-stream`.

## Backlog

- Namespace resolution as a *feature* — gap 1 closed rather than pinned — needs a rule and an ADR
  before any code; nothing schedules it (`crates/nvs-stdlib/src/xml.rs:120` § *Known gaps*).
- `Core\Xml`'s remaining gaps are the whole of what that crate owes here; the plan's `Open now`
  points at the goals directory for everything else.
