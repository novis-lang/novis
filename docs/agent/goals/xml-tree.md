---
milestone: M8
---
# Loop goal 29 — `Core\Xml`'s tree and stream, and the WHATWG parser over it

`Core\Xml` is one API replacing DOM, SimpleXML, XMLReader, XMLWriter, `xml_parser_*` and XSLTProcessor,
and it is **the largest single unowned item in the repository** — because it is not free-standing.
`rule:core-classes/html-parsing` is
titled *Unscheduled, and lands with `Core\Xml`*, and `crates/nvs-stdlib/src/html.rs`'s own *Known gaps*
says `Core\Html::sanitize` and the WHATWG parser "both wait on that tree existing at all". Three gaps,
one tree, one goal.

**It is M8's, not M9's** — `rule:core-api/tier-roster` puts `Core\Xml` at
Tier 0 (`0051:94`) and [m9.md](../../plan/m9.md) carries the extension system, not document formats.
`spec-classes-part-two-outstanding.txt`'s claim that M9 carried § 17 was wrong; goal `formats` corrected the
other half.

**It sits last of the three M8 entries** because it is the biggest and shares no file with them, so a
run that has to stop stops with the cheap two landed.

## Stage 0 — the catch-up

1. **`crates/nvs-stdlib/src/html.rs`'s *Known gaps*** states the wait explicitly and is rewritten when
   the wait ends — not amended with an overlay.
2. **`rule:core-classes/html-parsing` is the *Unscheduled* section**, and this entry existing is what makes it wrong. The
   fold is into that ADR's own body, naming this goal, per the rule that an ADR's body always states
   the current position.

## Stage 1 — the floor

Goal `formats`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the tree, and it is one node family

1. **One node family**, per `rule:core-classes/html-parsing`'s *One tree, two front doors*: element, text, comment,
   processing instruction, document. `Core\Html`'s parser and `Core\Xml`'s parser produce the same
   nodes, and that is the whole reason this goal is one goal.
2. **The tree materialises and the stream does not**, and no operation is available through both —
   spec § 17 states this outright as "the one place in this file where two shapes of the same subsystem
   coexist", so it is not read as an exception to R17. The module doc restates the split once and the
   member cards never re-argue it.
3. **A parsed tree is inert data**, on
   `rule:tooling/reflection-and-source-parsing-are-core-features`'s rule for the AST: no
   path back into execution, no entity expansion that reaches a filesystem or a network, and no
   `XSLTProcessor` shape at all.
4. **The three classic XML attacks are refused by construction**, per
   `rule:core-classes/xml-refuses-by-construction`: external entity resolution does not exist as a code
   path (not a flag that defaults to off), a DTD that names an external subset is refused rather than
   fetched, and a billion-laughs expansion is closed at its `<!DOCTYPE` rather than metered — the
   declaration is refused whole, so there is no internal subset to declare entities in and no expansion
   factor for goal `formats`'s ratio-and-ceiling rule to bound.
5. **A parsed value is `tainted`**, every string that comes out of it, on the ordinary rule that a
   return from a parser over untrusted bytes is untrusted.

## Stage 3 — the stream

1. **A reader and a writer**, replacing XMLReader and XMLWriter, holding one window rather than the
   document — which is the property that makes the split worth having.
2. **The reader answers the same node family** stage 2 declares, one node at a time, so a program that
   outgrows the tree does not learn a second vocabulary.
3. **The writer cannot emit ill-formed output.** Element nesting is enforced by the writer's own state,
   not by the caller remembering to close; an unclosed element at the end is an error, not a document.

## Stage 4 — the WHATWG parser, on `Core\Html`

1. **`html5ever` through a Novis-owned tree builder**, per `rule:core-classes/html-parsing` — the engine is the crate, the
   tree is ours, and the boundary between them is where stage 2's node family is enforced.
2. **Never-failing**, per the spec § 17 row: tag soup produces a document, because the WHATWG algorithm
   has no failure mode. A parser that can throw would make sanitizing untrusted markup conditional on
   the attacker's cooperation.
3. **It is an entry on `Core\Html`, never a mode of `Core\Xml`** — `rule:core-classes/html-parsing`, and the alternative it
   rejects is exactly the flag this must not become.

## Stage 5 — `Core\Html::sanitize`, which is why the tree matters

1. **`sanitize` is an `rule:security/tainted-qualifier` launderer**, and
   it is the *second* one on this class — `escape` is the auto-applied one. It parses with stage 4's
   parser, walks stage 2's tree against a closed allowlist of elements and attributes, and serialises.
2. **An allowlist, never a denylist**, and it is closed rather than configurable. A sanitizer whose
   policy the caller writes is a sanitizer whose bugs are the caller's, which is the mXSS history this
   is meant to end.
3. **mXSS is the acceptance property**: parse-sanitize-serialise-reparse must reach a fixed point, and
   the corpus that proves it is the one the case tree carries.
4. **`spec-classes-part-two-outstanding.txt` loses `§17 Core\Xml`** — the last of the eight keys, so
   that file holds none and spec §§ 16–17 is registered whole.

## Standing decisions

- **This goal may open one ADR number**, for `Core\Xml`'s own surface — the tree/stream split and the
  entity and expansion refusals. `rule:core-classes/html-parsing` is already written and gains only the § 4 fold; it is not
  reopened.
- **There is no HTML mode on the XML parser and no XML mode on the HTML one.** `rule:core-classes/html-parsing` decided
  this and its *Alternatives rejected* says why; a session that finds the two-parsers-one-tree shape
  inconvenient has found the shape working as intended.
- **No entity resolution reaches the outside world, ever** — not behind a capability, not behind a
  flag. There is no legitimate use that justifies the class of bug it opens, and a program that needs
  to fetch something fetches it with `Core\Http\Client` and parses the bytes.
- **`sanitize`'s allowlist is closed and is not configurable.** If a real application needs an element
  the list lacks, the list gains it in a commit with a reason, not a parameter.
- **Ambiguity about tree ergonomics resolves toward the narrower API**, recorded in the module doc:
  six extensions collapse into one surface, and the collapse is the point.
- **What this spends**, per `rule:programs/memory-priority`: a materialised tree is
  proportional to the document and is held for the request that parsed it, released with its arena.
  The stream holds one window. Both are bounded by goal `formats`'s ceiling when the input arrived compressed.
