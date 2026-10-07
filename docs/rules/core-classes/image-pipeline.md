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

A pipeline starts from one of four sources — encoded bytes (`open`), a blank canvas (`create`), RGBA8
pixels (`fromRaw`) or text set in a font (`measureText`) — and `run` returns one of three outputs: the
encoded file (`encode`), the RGBA8 pixels behind a size header (`raw`), or the size header alone
(`measureText`). An overlay is a row of the plan's flat overlay table that a `composite` step names by
position, because WIT has no recursive types. `hashDistance`, `Color` and `Font::fromBytes` are Novis
source and call no export.

**Not shipped.** The interface is written: `wit/image.wit` holds package `nvs:image@1.0.0`, its
`codec` interface and the `image` world, and `crates/nvs-stdlib/tests/ext_world.rs` places every
builder member. A plan's `step` is a WIT record with one optional field per operation, exactly one
of them set, because the manifest tells a union's cases apart by their required keys and most steps
have none: a step crosses as a Novis array keyed by its operation, `['resize' => [...]]`. The
component's `run` and `variants` take plans of `format` and `metadata` steps alone
(`extensions/image/src/lib.rs`); `variants` decodes once and costs one host-to-guest call, counted by
`nvs_ext::call::Request::crossings` (`crates/nvs-ext/tests/image_pipeline.rs`).
