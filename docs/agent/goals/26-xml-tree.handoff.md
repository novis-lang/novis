# Handoff

## State

**Goal 26 — `Core\Xml`'s tree and stream, and the WHATWG parser over it — has just started; nothing of
it has landed yet.** Goal 25's whole list is this goal's Stage 1 floor.

**This is the largest single unowned item in the repository**, because it is not free-standing.
`crates/nvs-stdlib/src/html.rs`'s *Known gaps* says `Core\Html::sanitize` and
[ADR 0122](../../adr/0122-html-parsing-is-a-whatwg-entry-on-core-html-over-core-xmls-tree.md)'s WHATWG
parser "both wait on that tree existing at all", and ADR 0122 § 4 is titled *Unscheduled, and lands
with `Core\Xml`*. Three gaps, one tree, one goal — and this entry existing is what makes that § 4
wrong, so the fold into ADR 0122's body is stage 0's work.

## Next group

**Stage 2: the tree** — one file set: the new `crates/nvs-stdlib/src/xml.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- [ ] **One node family** — element, text, comment, processing instruction, document (ADR 0122 § 2).
      `Core\Html`'s parser and `Core\Xml`'s produce the same nodes, which is the whole reason this is
      one goal rather than two.
- [ ] **The tree materialises and the stream does not, and no operation is available through both.**
      Spec § 17 states this as the one place two shapes of a subsystem coexist, so it is not read as an
      exception to R17. Say it once in the module doc; member cards never re-argue it.
- [ ] **A parsed tree is inert data**, on ADR 0019's rule for the AST: no path back into execution, no
      `XSLTProcessor` shape at all.
- [ ] **The three classic attacks are refused by construction** — external entity resolution is *not a
      code path* rather than a flag defaulting to off, billion-laughs is bounded by goal 25's ceiling,
      and a DTD naming an external subset is refused rather than fetched.
- [ ] **Every string out of a parsed tree is `tainted`.**

## Backlog

- **Stage 3 (the stream)** is a reader and a writer holding one window rather than the document — the
  property that makes the split worth having. The writer enforces nesting from its own state, so an
  unclosed element at the end is an error and not a document.
- **Stage 4 (the parser)** is `html5ever` through a Novis-owned tree builder (ADR 0122 § 3), and it is
  **never-failing**: tag soup produces a document, because a parser that can throw makes sanitizing
  untrusted markup conditional on the attacker's cooperation. It is an entry on `Core\Html`, never a
  mode of `Core\Xml` (§ 1).
- **Stage 5 (`sanitize`)** is the second ADR 0024 launderer on `Core\Html` — parse, walk a **closed**
  allowlist, serialise. The acceptance property is mXSS: parse-sanitize-serialise-reparse reaches a
  fixed point. It also strikes the last of the eight keys, so
  `spec-classes-part-two-outstanding.txt` holds none and spec §§ 16–17 is registered whole.
