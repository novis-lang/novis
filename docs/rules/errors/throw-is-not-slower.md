The marginal cost of `rule:errors/propagation`'s status check is **0.85 ns per frame**, measured as
the slope between a 2-frame and an 18-frame chain so the harness's own call-out overhead cancels. A
single L2 miss is roughly 10 ns. The branch is perfectly predicted on the happy path, so what is
actually spent is the instruction slot.

**A throw costs slightly less than a normal return** — 0.79–0.82× at depth 8 — because the error
path returns the status immediately while the success path also copies a 16-byte value up through
every frame. That is the property PHP compatibility rests on, since frameworks throw on ordinary
control-flow paths.

It holds only while *propagating* does not allocate. Storing a message as a `String` made a throw
2.8× a return, more than the whole propagation path it was meant to measure, so the pending-error
slot is a `Cow<'static, str>` and a static exception message allocates nothing.

**The raise itself renders one frame label, and that is the whole of what a throw allocates.** The
exception carries the frame it was raised in, rendered from the site the `throw` was compiled with,
so a `catch` beside the `throw` — the one place no frame is ever unwound out of — reads a backtrace
naming that frame instead of an empty one. It is spent **once per raise and never per frame**, which
is what leaves the slope above untouched, and the label the frame pushes as the throw leaves
replaces that rendering rather than following it, so no frame is named twice. **A checked operator
is handed its statement's site on the same terms**: the blob is baked in the cold block it already
raises from, beside the message bytes, so the arithmetic that does not overflow spends no
instruction on it. **A helper's fault is handed no site**, and its frame is rendered on the one
edge that would otherwise carry none — reaching a `catch` in the frame the helper was called from.
That rendering sits on the caught edge of the landing site, which only a failure enters, so a call
that returned cleanly spends nothing on it, and it writes only where nothing already named a frame,
so an exception arriving from a callee passes through untouched. The bound: a fault that
*propagates* out of the frame it was raised in opens its trace at that frame's own label as before
and names no `location`, a pushed label being a rendering rather than the datum a `location` is read
back from.
