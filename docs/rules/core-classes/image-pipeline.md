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
A call is a fixed cost and a bulk copy is memcpy-bound, and a pixel buffer is the largest thing a
request holds — a 24-megapixel frame is 96 MB at RGBA8 — so a request pays one instance and one
crossing per terminal, and the pixel buffer never leaves the guest. The builder grows a member
without touching the ABI, because a new operation is a new plan step rather than a new export.

**An animation moves through the plan as one value.** Every step runs on every frame, in the order
written, and the frames keep their delays. Only a GIF output keeps the frames: `encode` to GIF writes
every one with its delay, every other target writes the first frame, and `raw()` returns the first
frame. A plan whose output is not GIF decodes the first frame alone, so a thumbnail of an animation
costs one frame's decode, as a still image does. The pixel cap counts every frame the header states
either way (`rule:core-classes/image-pixel-cap`), so a request's pixel memory is bounded by the cap
however many frames it keeps. Every frame keeps one size: `trim` finds its box on the first frame
and crops every frame to it, and a `composite` overlay is made once and drawn on each. A GIF output
loops forever, since the decoders do not return the input's loop count.

A pipeline starts from one of four sources — encoded bytes (`open`), a blank canvas (`create`), RGBA8
pixels (`fromRaw`) or text set in a font (`measureText`) — and `run` returns one of three outputs: the
encoded file (`encode`), the RGBA8 pixels behind a size header (`raw`), or the size header alone
(`measureText`). An overlay is a row of the plan's flat overlay table that a `composite` step names by
position, because WIT has no recursive types. `hashDistance`, `Color` and `Font::fromBytes` are Novis
source and call no export.

`fromRaw` and `measureText` are pipeline starts, so each is a `source` case and not an export of its
own: both stay inside `run`, and a text source can be resized, padded and composited like any other
start. A `fromRaw` buffer that is not exactly `width * height * 4` bytes returns `invalid`, and the
pixel cap applies to `width * height` as it does to a header's declared size. A text source lays the
text out exactly as a `text` step does, so the size `measureText` returns is the box that step draws;
it rasterises the text to size it, which costs one glyph run. `run` returns `list<u8>` for every
output: the size header is width, then height, each a big-endian `u64`, and the host reads it into
the `Raw` shape or the size it returns. `hashDistance` counts the differing bits between two hashes
of one kind, which needs no decoder. A row's own `composite` steps name only earlier rows, so the
table is in dependency order and no overlay contains itself; the guest returns `invalid` for a
position that is not earlier, and the host flattens a nested Novis pipeline into rows when it builds
the plan.

- **Memory:** the `raw` output adds 16 bytes to the pixel buffer it returns.

A `Format` carries no members, because an enum declares only cases (`rule:enums/no-class-machinery`):
`Image::mime(Format)` and `Image::extension(Format)` name a format's MIME type and file extension. The
plan's option shapes that the builder fills in for the caller — `format`'s, `rotate`'s and
`composite`'s — declare `x: ?T` rather than `x?: T` in `extensions/image/manifest.json`, because a
Novis shape has no way to leave a key out conditionally; both cross as the same WIT `option`.

**Partly on disk.** The interface is written: `wit/image.wit` holds package `nvs:image@1.0.0`, its
`codec` interface and the `image` world, and `crates/nvs-stdlib/tests/ext_world.rs` places every
builder member. A plan's `step` is a WIT record with one optional field per operation, exactly one
of them set, because the manifest tells a union's cases apart by their required keys and most steps
have none: a step crosses as a Novis array keyed by its operation, `['resize' => [...]]`. The
component's `run` and `variants` run every step, in order, and check each step's
options before anything is decoded; what each pixel step does beyond this rule, and the
pixel cap every frame a step makes is held to, are `extensions/image/src/ops.rs`'s module doc. A
`composite` step names only an earlier row, and one plan draws a bounded number of overlays counting
every level, so a few rows that each draw the one before twice cannot double the work per level
(`extensions/image/src/lib.rs` module doc). The builder's `composite` appends the overlay's own
rows and then the overlay to the table, renumbering the overlay's steps by the rows already there. `variants` decodes once and costs one host-to-guest call, counted by
`nvs_ext::call::Request::crossings` (`crates/nvs-ext/tests/image_pipeline.rs`). The `Novis\Image`
builder is Novis source in `extensions/image/nvs/`, packed into the component's `nvs.source`:
`Image` with every member `IMAGE_MEMBERS` in `crates/nvs-stdlib/tests/ext_world.rs` places, and
`Color` with `rgba` and `hex` and no named constants yet (`tests/conformance/novis/`). A canvas or
pixel source is held to the default pixel cap, not the `[image]` one. A plan that encodes a GIF
runs every step on every frame of a GIF, an APNG and an animated WebP and writes them with their
delays (`extensions/image/src/lib.rs`, `extensions/image/src/encode.rs`), and any other plan
decodes the first frame alone (`crates/nvs-ext/tests/image_pipeline.rs`).

The image world is called typed (`crates/nvs-ext/src/typed/image.rs`), so decoded pixels cross the
boundary in one copy each way, and their host bytes are charged to the request at their own size. A
`raw()` too large for the request stops at its memory limit (`crates/nvs-ext/src/call.rs`'s
`Request::run`).

**Declined:**
- **A gd-shaped handle with a call per operation.** It materialises every intermediate, prevents
  fusion, and either mutates in place, which no other Novis value does, or copies a frame per
  operation.
- **The resampling kernel at Tier 2, only the codecs in the sandbox.** It moves the pixel buffer
  across the boundary twice per pipeline, which costs more than the wasm penalty saves, and splits
  one job across two tiers. If a committed benchmark puts in-guest resampling far below native for a
  real workload, the exit is Tier 2 for that kernel alone, never `dlopen`.
- **ImageMagick or libvips compiled to wasm wholesale.** A surface that large is the long tail the
  component declines to own, and a C codebase that size is still one to build and audit inside the
  sandbox.
- **An export of its own for `fromRaw` and for `measureText`.** Each would carry the same input as a
  `source` case, add a boundary entry point for a job that is one more pipeline start, and break the
  closed export count.
- **A `variant` result for `run`.** It changes `run`'s signature to carry one case that two members
  use, where a size header on the same `list<u8>` carries it.
- **`hashDistance` as a call into the component.** It pays the boundary for a loop over a few bytes.
