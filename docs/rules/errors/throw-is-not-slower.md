The marginal cost of `rule:errors/propagation`'s status check is **0.85 ns per frame**, measured as
the slope between a 2-frame and an 18-frame chain so the harness's own call-out overhead cancels. A
single L2 miss is roughly 10 ns. The branch is perfectly predicted on the happy path, so what is
actually spent is the instruction slot.

**A throw costs slightly less than a normal return** — 0.79–0.82× at depth 8 — because the error
path returns the status immediately while the success path also copies a 16-byte value up through
every frame. That is the property PHP compatibility rests on, since frameworks throw on ordinary
control-flow paths.

It holds only while throwing does not allocate. Storing a message as a `String` made a throw 2.8× a
return, more than the whole propagation path it was meant to measure, so the pending-error slot is
a `Cow<'static, str>` and a static exception message allocates nothing.
