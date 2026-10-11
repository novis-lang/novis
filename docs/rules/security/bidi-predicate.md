Nine code points open or close a directional scope: four embedding and override forms closed by one
terminator, and three isolate forms closed by another. Two more are marks rather than scopes and are
**not** covered, because they open nothing and therefore cannot be unterminated.

| Code points | Role | Closed by |
|---|---|---|
| U+202A `LRE`, U+202B `RLE`, U+202D `LRO`, U+202E `RLO` | embedding and override, the legacy forms every published exploit uses | U+202C `PDF` |
| U+2066 `LRI`, U+2067 `RLI`, U+2068 `FSI` | isolate, the forms Unicode recommends | U+2069 `PDI` |
| U+200E `LRM`, U+200F `RLM` | mark: changes one character's resolved direction and opens no scope | *not covered* |

**A span is rejected when it ends with a scope still open.** Two independent counters, one for
embeddings and overrides and one for isolates: an opener increments, the matching terminator
decrements with a floor of zero, and both must be zero at the end of the span. A stray terminator
with no opener closes nothing and opens nothing, so it is accepted.

The attack is Trojan Source: text that renders in an order its bytes do not have, so a reviewer
approves one program and the compiler builds another. Code review is the control being defeated, and
the published exploits live in comments and string literals. Identifiers are ASCII-only, which
closes homoglyph and bidi identifiers structurally; what is left is comments, string literals,
inline-HTML text and runtime data, and the condition is the same in all four. The line is drawn at
*unterminated*, not at *present*, the line other compilers draw too, because balanced controls are
what legitimate Arabic, Hebrew and mixed-direction text uses.

This is deliberately simpler than the full bidirectional algorithm, whose directional status stack has
interactions the counters flatten. That is correct here because the question is not *how does this
render* — which needs the real algorithm — but *does this span leave a scope open*, which the counters
answer soundly. The simplification can only over-approximate towards rejecting, which is the safe
direction. It costs nothing measurable: two counters carried through a walk that happens anyway, no
allocation and no table lookup.

**Declined:**
- **Rejecting the eleven code points outright, terminated or not.** It breaks legitimate
  mixed-direction literals: an Arabic message interpolating a Latin name needs an isolate to render
  correctly. It also pushes developers to build such strings by concatenation to get past the check.
- **The full bidirectional algorithm.** The question is whether a scope is left open, not how the
  span renders (`rule:security/bidi-scope-is-the-whole-target`).
