An `Image` is an **immutable value**: the source bytes plus a plan. `open` reads the header and
decodes nothing; every operation returns a new value that shares the bytes and extends the plan, so
branching a pipeline costs an array rather than a frame; and nothing decodes until a terminal —
`encode`, `variants`, `raw` — runs it. The plan executes in the order written, and steps may be fused
only where the result differs by resampling rounding alone.

That retires gd's mutable resource and every ambient mode flag with it: alpha always composites and
always saves, the resampling filter is an option on `resize`, and progressive encoding is an option
on `format`.

**The component's entry points are few, coarse and closed**, each one host-to-guest call carrying its
whole input and returning its whole output. There is no per-pixel, per-row or per-frame member, and
none is added later: `raw()` exports the whole buffer once for the caller that genuinely needs
pixels. The rule for any future entry point is a refusal — a member that would be called in a loop
over the image's own contents is not admitted, because its cost is the boundary rather than the work.

**Not shipped.** There is no image extension in the tree: no `nvs/image` package and no crate behind
it.
