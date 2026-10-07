An `Image` is an **immutable value**: the source bytes plus a plan. `open` keeps the bytes and calls
nothing, so bytes that are not an image throw at the terminal, and a pipeline costs one host-to-guest
call however it was built; every operation returns a new value that shares the bytes and extends the plan, so
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

A `Format` carries no members, because an enum declares only cases (`rule:enums/no-class-machinery`):
`Image::mime(Format)` and `Image::extension(Format)` name a format's MIME type and file extension. The
plan's option shapes that the builder fills in for the caller — `format`'s, `rotate`'s and
`composite`'s — declare `x: ?T` rather than `x?: T` in `extensions/image/manifest.json`, because a
Novis shape has no way to leave a key out conditionally; both cross as the same WIT `option`.

**Not shipped.** The interface is written: `wit/image.wit` holds package `nvs:image@1.0.0`, its
`codec` interface and the `image` world, and `crates/nvs-stdlib/tests/ext_world.rs` places every
builder member. A plan's `step` is a WIT record with one optional field per operation, exactly one
of them set, because the manifest tells a union's cases apart by their required keys and most steps
have none: a step crosses as a Novis array keyed by its operation, `['resize' => [...]]`. The
component's `run` and `variants` run every step but `text`, in order, and check each step's
options before anything is decoded; what each pixel step does where 0120 leaves it open, and the
pixel cap every frame a step makes is held to, are `extensions/image/src/ops.rs`'s module doc. A
`composite` step names only an earlier row, and one plan draws a bounded number of overlays counting
every level, so a few rows that each draw the one before twice cannot double the work per level
(`extensions/image/src/lib.rs` module doc). The builder's `composite` appends the overlay's own
rows and then the overlay to the table, renumbering the overlay's steps by the rows already there. `variants` decodes once and costs one host-to-guest call, counted by
`nvs_ext::call::Request::crossings` (`crates/nvs-ext/tests/image_pipeline.rs`). The `Novis\Image`
builder is Novis source in `extensions/image/nvs/`, packed into the component's `nvs.source`:
`Image` with every member of 0120 § 2's table but `text`, and `Color` with `rgba` and `hex` and no named constants yet
(`tests/conformance/novis/`). A canvas or pixel source is held to the default pixel cap, not the
`[image]` one.
