Nine code points open or close a directional scope: four embedding and override forms closed by one
terminator, and three isolate forms closed by another. Two more are marks rather than scopes and are
**not** covered, because they open nothing and therefore cannot be unterminated.

**A span is rejected when it ends with a scope still open.** Two independent counters, one for
embeddings and overrides and one for isolates: an opener increments, the matching terminator
decrements with a floor of zero, and both must be zero at the end of the span.

This is deliberately simpler than the full bidirectional algorithm, whose directional status stack has
interactions the counters flatten. That is correct here because the question is not *how does this
render* — which needs the real algorithm — but *does this span leave a scope open*, which the counters
answer soundly. The simplification can only over-approximate towards rejecting, which is the safe
direction. It costs nothing measurable: two counters carried through a walk that happens anyway, no
allocation and no table lookup.
