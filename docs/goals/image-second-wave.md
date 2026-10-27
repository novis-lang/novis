---
looper: 1
title: "AVIF and JPEG XL open upright and in sRGB, `Image::info` reads EXIF, and an animated image has every frame"
summary: "An AVIF and a JPEG XL input are oriented and converted to sRGB as the other formats are, Image::info returns the exif fields a file carries, an animated input of each animated format the roster decodes reports its real frame count and decodes every frame, every designed rule this goal owns is shipped, and the checks below pass."
context: [docs/rules/core-classes/image-correct-by-default.md, docs/rules/core-classes/image-format-roster.md, docs/rules/core-classes/image-pixel-model.md, docs/rules/core-classes/image-pixel-cap.md, docs/rules/core-api/tier-roster.md]
stages:
  - id: 1
    title: "an AVIF and a JPEG XL input open upright and in sRGB"
    status: done
    context: [extensions/image/src/decode.rs, extensions/image/src/avif.rs, crates/nvs-ext/tests/image_pipeline.rs, extensions/image/fixtures/README.md]
  - id: 2
    title: "`Image::info` returns the `exif` fields a file carries"
    status: done
    context: [extensions/image/src/info.rs, crates/nvs-ext/tests/image_decode.rs]
  - id: 3
    title: "`Image::info`'s `frames` is an animated input's real count"
    status: done
    context: [extensions/image/src/info.rs, extensions/image/src/avif.rs, crates/nvs-ext/tests/image_decode.rs]
  - id: 4
    title: "the rules say what an animation costs, and GIF, APNG and WebP decode every frame"
    status: done
    context: [extensions/image/src/decode.rs, extensions/image/src/lib.rs, docs/rules/core-classes/image-pipeline.md]
  - id: 6
    title: "the plan runs every step on every frame, and a GIF output keeps them"
    status: done
    context: [extensions/image/src/lib.rs, extensions/image/src/ops.rs, extensions/image/src/encode.rs, crates/nvs-ext/tests/image_pipeline.rs]
  - id: 7
    title: "an animated AVIF and JPEG XL decode every frame"
    status: done
    context: [extensions/image/src/avif.rs, extensions/image/src/decode.rs, crates/nvs-ext/tests/image_decode.rs]
  - id: 5
    title: "the four rules are shipped, and the roster says the component is whole"
    status: open
    context: [docs/rules/core-api/tier-roster.md]
  - id: 8
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: image-proofs
    stage: 5
    name: the members this goal changes owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "Novis\\Image\\Image::open", "Novis\\Image\\Image::info"]
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, image-second-wave]
    reach: true
    floor: false
---

# AVIF and JPEG XL open upright and in sRGB, `Image::info` reads EXIF, and an animated image has every frame

## Why

The image component is built into `nvs` and always on (`rule:packaging/the-first-party-components-are-built-in`).
Three of its rules are only partly on disk. `open` orients and converts to sRGB the formats `image`
decodes, but an AVIF or JPEG XL frame comes back as `rav1d` and `jxl-oxide` give it
(`extensions/image/src/decode.rs:21`, `rule:core-classes/image-correct-by-default`). `Image::info`
returns `exif` as `null` and `frames` as `1`, because the component has no EXIF reader and counts no
frames (`extensions/image/src/info.rs:5`, `rule:core-classes/image-pixel-model`). An animated input
decodes its first frame only (`extensions/image/src/decode.rs:16`,
`rule:core-classes/image-format-roster`). This goal builds all three, and `rule:core-api/tier-roster`
stops naming the gap.

## Standing decisions

**The defaults are the rule's, for every format.** AVIF and JPEG XL take the same two `open` options as
the formats `image` decodes, on by default, and `encode` strips what they carried
(`rule:core-classes/image-correct-by-default`). A profile that does not parse is ignored, as it is for
the others. AVIF's orientation is its `irot` and `imir` boxes, which browsers follow; an EXIF
orientation inside an AVIF is not applied a second time. JPEG XL's is the codestream's own field.

**The EXIF reader is `kamadak-exif`, already a dependency** (`extensions/image/Cargo.toml:30`,
used by `check_exif` at `extensions/image/src/decode.rs:105`). No crate is added. JPEG, PNG and WebP
give their block through `image`'s `exif_metadata`; AVIF's `Exif` item and JPEG XL's `Exif` box are read
by a bounded box walk in the component when `avif-parse` or `jxl-oxide` does not expose them. Each
field is `(tag name, display value)` as `kamadak-exif` names and prints it, in file order, MakerNote
left out. Why: the dependency is already reviewed and in the guest; a second reader adds attack surface.

**`exif` lists the image's own IFDs, and a text value is its bare text.** The thumbnail's IFD is
left out, because its tags repeat the image's names and describe a picture `info` never returns.
A text value is the text the file stores, so a date is `2024:01:02 03:04:05`, without the quotes
`kamadak-exif` prints; every other value is `kamadak-exif`'s display with its unit. AVIF and JPEG
XL's `info` read their own header: AVIF through `avif-parse` and the box walk, whose `irot` and
`imir` give the EXIF orientation they amount to, and JPEG XL through `jxl-oxide`'s `aux_boxes`.
Why: a program reads the date it would read in PHP or a viewer, and quoted text is a second
parse for every caller.

**`exif` is read from the header, and a malformed block is a `Parse` error**, as `decode` already
treats one (`extensions/image/src/decode.rs:105`). `info` still decodes no pixel.

**`frames` is counted without decoding a frame.** Each container states its frame count or lists its
frames, so stage 3 reads it from the header and costs no pixel memory.

**An animation's cost is the sum of its frames.** `rule:core-classes/image-pixel-cap` holds width
times height times `frames` to `[image] max_pixels`, read off the header before any frame decodes.
Every plan step runs on every frame, and the frames move through the plan together. `encode` to GIF
writes every frame with its delay; every other target writes the first frame, and a plan whose
output is not GIF decodes the first frame only. `raw()` returns the first frame. Why: security first,
since one cap bounds a request however many frames a file claims, where a per-frame cap lets a
thousand-frame file take a thousand frames; then latency, since a thumbnail of an animation decodes
one frame as it does now. Stage 4 writes this into `image-pixel-cap` and `image-pipeline`.

**Fixtures are made with ImageMagick 7 and ffmpeg, and each command is recorded** in
`extensions/image/fixtures/README.md`, as the existing ones are: 16 by 12 pixels, a few frames, no
personal data. `magick` writes the orientation (`-orient RightTop`), the profile (`-profile`), the
EXIF block and the animated GIF, APNG (`apng:`), WebP and JPEG XL; ffmpeg's `avif` muxer writes the
animated AVIF if `magick` writes a still one. The RGB profile is a Display P3 profile `moxcms` writes,
so no third-party profile is committed. Each fixture is checked with `magick identify -verbose` before
it is committed. A broken input is built by the test itself, as today.

**Trade-offs.** A program gains: a phone photo in AVIF or JPEG XL is upright and in the right colours,
and `info` returns the tags it used to need another library for. Request-path cost is one EXIF parse
per `info` call and one colour transform per AVIF or JPEG XL `open`, both on work the call already
does. Memory for stages 1 to 3 is one extra frame during the sRGB conversion, as the other formats
already pay, per request, freed with it. An animation spends at most what one frame at the cap spends
today, per request, because the cap counts every frame.

**An animation keeps one frame size, and a GIF output loops forever.** `trim` finds its box on the
first frame and crops every frame to it, and a `composite` overlay is made once and drawn on every
frame, so a GIF's frames all have its first frame's size. The decoders do not return the input's
loop count, so `encode::animation` writes `Repeat::Infinite`. `variants` decodes every frame only when
one of its plans encodes a GIF, and each plan that does not runs on a copy of the first frame alone.
The per-step cap counts the frames the plan runs on, through `ops::apply` and `ops::size`'s `frames`
argument, so a `resize` that grows three frames is held to three times its size. Why: a trim per
frame would give frames of different sizes, which a GIF's one logical screen cannot place without a
second offset model; one overlay decode per step, not per frame, is the latency the pipeline already
pays for a still image.

**An AVIF sequence's frames are its track's samples, under the primary item's properties.** Every
frame of `decode_frames` comes from the `pict` track through one `rav1d` decoder, its alpha from an
`auxv` track through a second, and the primary item's `irot`, `imir` and `colr` apply to each frame.
The sample tables are walked as the frames decode, so they allocate nothing, and `stsc` is read
forward once. A frame of another size than the first is `Parse`. Why: security first, since a
table walk that allocates per entry, or rescans `stsc` per chunk, lets a small file cost memory or
quadratic time; then simplicity, since the frames share one orientation and profile, as the first
frame's `decode` already applies them, and the track's own `tkhd` matrix is not read.

**Lanes.** The whole goal touches only `extensions/image/`, `crates/nvs-ext/tests/image_*.rs` and the
image rules, so it can run as a side goal beside the chain now. Stages 1 and 2 touch disjoint files
(`decode.rs`, `avif.rs` against `info.rs`) and run in either order. Stage 3 shares `info.rs` with stage
2 and runs after it. Stages 4, 6 and 7 share `decode.rs` and `lib.rs`, after stages 1 and 3, in that
order. Stage 5 is last.

## Stages

### 1. an AVIF and a JPEG XL input open upright and in sRGB

**Does:** an AVIF and a JPEG XL fixture with an orientation and an embedded profile decode upright
and in sRGB, and each option set to `false` returns the raw frame.

**Split:** before the code, one writing subagent makes every fixture of stages 1 to 3 in one pass, as
the standing decision says, and records each command in the README. It touches only
`extensions/image/fixtures/`.

- [x] The fixtures: `oriented.avif`, `oriented.jxl` (orientation 6 and the P3 profile), `exif.png`, `exif.webp`, `exif.avif`, `exif.jxl`, and an animated `anim.gif`, `anim.png`, `anim.webp`, `anim.avif`, `anim.jxl` of three frames, each command in the README. `extensions/image/fixtures/README.md:1`
- [x] AVIF: `irot` and `imir` applied with `auto_orient`, and an ICC `colr` box converted through `srgb` with `to_srgb` (`rule:core-classes/image-correct-by-default`). `extensions/image/src/avif.rs:43`, `extensions/image/src/decode.rs:232`
- [x] JPEG XL: the codestream orientation applied, and the embedded profile converted through `srgb`; an XYB image with no profile asks `jxl-oxide` for sRGB output. `extensions/image/src/decode.rs:304`
- [x] The module doc's AVIF and JPEG XL sentences say what the code now does. `extensions/image/src/decode.rs:19`
- [x] Tests: each format opens upright and in sRGB, and each option set to `false` returns the raw frame, beside the JPEG case. `crates/nvs-ext/tests/image_pipeline.rs:427`

### 2. `Image::info` returns the `exif` fields a file carries

**Does:** `info` on a JPEG, PNG, WebP, AVIF and JPEG XL fixture with an EXIF block returns its fields,
`null` for a file with none, and a `Parse` error for a malformed block.

- [x] The EXIF block of each format, read from the header as the standing decision says, into `Info::exif` (`rule:core-classes/image-pixel-model`). `extensions/image/src/info.rs:31`, `extensions/image/src/info.rs:70`
- [x] The module doc says what `exif` holds now. `extensions/image/src/info.rs:5`
- [x] Tests: the five tagged fixtures give their fields, `gradient.png` gives `null`, and a malformed block built by the test is `Parse`. `crates/nvs-ext/tests/image_decode.rs:185`

### 3. `Image::info`'s `frames` is an animated input's real count

**Does:** an animated fixture of each animated format the roster decodes (APNG, GIF, WebP, AVIF and
JPEG XL) gives its frame count, and a still image gives `1`.

- [x] The count from each container's header: APNG `acTL`, GIF's image descriptors, WebP `ANMF` chunks, AVIF's image sequence track, JPEG XL's frame headers, with no pixel decoded. `extensions/image/src/info.rs:68`, `extensions/image/src/avif.rs:43`
- [x] Tests: each `anim.*` fixture gives `3`, and each `gradient.*` gives `1`. `crates/nvs-ext/tests/image_decode.rs:185`

### 4. the rules say what an animation costs, and GIF, APNG and WebP decode every frame

**Does:** the two rules state the standing decision, the cap counts every frame, and the three formats
`image` decodes give every frame.

- [x] `rule:core-classes/image-pixel-cap` says the cap counts the sum of the frames, and `rule:core-classes/image-pipeline` says every step runs on every frame and which outputs keep them. `docs/rules/core-classes/image-pixel-cap.md:5`, `docs/rules/core-classes/image-pipeline.md:13`
- [x] The cap check takes `frames` from the header and holds width times height times frames to the cap before any frame decodes (`rule:core-classes/image-pixel-cap`). `extensions/image/src/decode.rs:83`, `extensions/image/src/lib.rs:233`
- [x] GIF, APNG and WebP decode every frame through `image`'s animation decoders, with each frame's delay kept. `extensions/image/src/decode.rs:152`
- [x] Tests: an animation whose frames together pass the cap is refused before a frame decodes, and each `anim.*` of the three formats decodes three frames. `crates/nvs-ext/tests/image_decode.rs:185`

### 6. the plan runs every step on every frame, and a GIF output keeps them

**Does:** a pipeline over an animation applies each step to every frame, a GIF output writes them all
with their delays, and any other output decodes and writes the first frame only.

- [x] The plan runs over a list of frames, with the size check per step counting every frame. `extensions/image/src/lib.rs:170`, `extensions/image/src/lib.rs:262`, `extensions/image/src/ops.rs:274`
- [x] A plan whose output is not GIF decodes the first frame only. `extensions/image/src/lib.rs:278`
- [x] `encode` to GIF writes every frame with its delay. `extensions/image/src/encode.rs:34`
- [x] Tests: a resized `anim.gif` keeps three frames, and the same plan to PNG decodes one frame. `crates/nvs-ext/tests/image_pipeline.rs:63`

### 7. an animated AVIF and JPEG XL decode every frame

**Does:** the AVIF image sequence and a JPEG XL animation decode every frame under the same cap.

- [x] AVIF's sequence track decodes frame by frame through `rav1d`. `extensions/image/src/avif.rs:43`
- [x] JPEG XL renders every frame through `jxl-oxide`. `extensions/image/src/decode.rs:304`
- [x] The module doc no longer says only the first frame is decoded. `extensions/image/src/decode.rs:16`
- [x] Tests: `anim.avif` and `anim.jxl` decode three frames, and a resized one to GIF keeps them. `crates/nvs-ext/tests/image_decode.rs:185`

### 5. the four rules are shipped, and the roster says the component is whole

**Does:** the rules lose their *Partly on disk* lines about this work, each record is `shipped` with its
tests in `guardedBy`, and the changed members owe no proof.

**Split:** the about pages and examples of `open` and `info` go to one writing subagent, one member
after the other, held to § *Text an end user reads* in `AGENTS.md`.

- [ ] `rule:core-classes/image-correct-by-default` loses its AVIF and JPEG XL gap and is `shipped`. `docs/rules/core-classes/image-correct-by-default.md:18`, `data/rules/core-classes/image-correct-by-default.json:3`
- [ ] `rule:core-classes/image-format-roster` loses its first-frame sentence and is `shipped`. `docs/rules/core-classes/image-format-roster.md:23`, `data/rules/core-classes/image-format-roster.json:3`
- [ ] `rule:core-classes/image-pixel-model` loses its `exif` sentence and is `shipped`. `docs/rules/core-classes/image-pixel-model.md:32`, `data/rules/core-classes/image-pixel-model.json:3`
- [ ] `rule:core-api/tier-roster` stops naming the image gap and is `shipped`. `docs/rules/core-api/tier-roster.md:36`, `data/rules/core-api/tier-roster.json:3`
- [ ] `image-pixel-cap` and `image-pipeline` stay `shipped` with stage 4's and 6's tests in `guardedBy`. `data/rules/core-classes/image-pixel-cap.json:3`, `data/rules/core-classes/image-pipeline.json:3`
- [ ] `open` and `info` show an AVIF photo and an animation in an example each, and `bun nv proofs --id 'Novis\Image\Image::info'` and `--id 'Novis\Image\Image::open'` owe nothing. `docs/examples/novis/Image-Image/info/about.md:1`, `docs/examples/novis/Image-Image/open/about.md:1`

### 8. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 to 4, 6 and 7 are done. `nvs_image::decode_frames` gives every frame of every animated
format: GIF, APNG and WebP through `image`, JPEG XL through `decode_jxl(.., every)`, which renders
each keyframe with its duration over the tick rate, and AVIF through `avif::decode_frames`, which
walks the `pict` track's samples (`Track`, `Samples`) through one `Stream` decoder and the `auxv`
track through a second. The standing decision *An AVIF sequence's frames are its track's samples*
records the choices. `decode` still decodes the first frame alone.

The rules' sentences that named AVIF and JPEG XL animation as first-frame-only are rewritten:
`image-format-roster`'s *Partly on disk*, `image-pipeline`'s, and `tier-roster`'s, which no longer
names this goal. None is `shipped` yet.

The sweep's findings were stale again: the `env-secret-proofs` log shows exit 0, and nightly run
`40291801939` is on `d6f131752`, before the fixes.

No test is expected red. The two changed tests passed once by name:
`an_animation_is_capped_by_all_its_frames_and_decodes_each_one` (now with `avif` and `jxl`, AVIF
held to 8 per channel because it is lossy) and
`a_resized_animation_keeps_every_frame_in_a_gif_and_its_first_in_a_png` (now resizing `anim.avif`
and `anim.jxl` to a GIF). The workspace clippy is green. `extensions/image`'s own `cargo clippy`
runs a newer lint set that is red on code across the crate (`chunks_exact`), which is not the gate.

### Next

**Stage 5: the four rules are shipped, and the roster says the component is whole.** Files: the
rule fragments and their `data/rules/` records, and the `open` and `info` example pages. Its
checks are `image-proofs` and `owners-closes`.

- [ ] Each of `image-correct-by-default`, `image-format-roster`, `image-pixel-model` and `tier-roster` drops the *Partly on disk* gap this goal closed, and its record goes from `designed` to `shipped` with the goal's tests in `guardedBy`. `docs/rules/core-classes/image-correct-by-default.md:21`, `docs/rules/core-classes/image-format-roster.md:18`, `docs/rules/core-classes/image-pixel-model.md:32`, `docs/rules/core-api/tier-roster.md:36`, `data/rules/core-classes/image-format-roster.json:3`
- [ ] `image-pixel-cap` and `image-pipeline` keep `shipped`, with stage 4's, 6's and 7's tests in `guardedBy`. `data/rules/core-classes/image-pixel-cap.json:3`, `data/rules/core-classes/image-pipeline.json:3`
- [ ] The examples of `open` and `info` (one writing subagent, held to § *Text an end user reads*), then `bun nv proofs --id` for each owes nothing. `docs/examples/novis/Image-Image/info/about.md:1`, `docs/examples/novis/Image-Image/open/about.md:1`
- [ ] `looper check --name image-proofs` and `bun nv owners --closes image-second-wave` are green. `tools/nv/cmd/owners.ts:1`

### Backlog

- Stage 8 is the gate: `bun nv affected --run` over the whole goal, every failure fixed in one pass.
