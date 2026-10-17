---
looper: 1
title: "AVIF and JPEG XL open upright and in sRGB, `Image::info` reads EXIF, and an animated image has every frame"
summary: "An AVIF and a JPEG XL input are oriented and converted to sRGB as the other formats are, Image::info returns the exif fields a file carries, an animated input of each animated format the roster decodes reports its real frame count and decodes every frame, every designed rule this goal owns is shipped, and the checks below pass."
context: [docs/rules/core-classes/image-correct-by-default.md, docs/rules/core-classes/image-format-roster.md, docs/rules/core-classes/image-pixel-model.md, docs/rules/core-classes/image-pixel-cap.md, docs/rules/core-api/tier-roster.md]
stages:
  - id: 1
    title: "an AVIF and a JPEG XL input open upright and in sRGB"
    status: open
    context: [extensions/image/src/decode.rs, extensions/image/src/avif.rs, crates/nvs-ext/tests/image_pipeline.rs]
  - id: 2
    title: "`Image::info` returns the `exif` fields a file carries"
    status: open
    context: [extensions/image/src/info.rs, crates/nvs-ext/tests/image_decode.rs]
  - id: 3
    title: "`Image::info`'s `frames` is an animated input's real count"
    status: open
    context: [extensions/image/src/info.rs, extensions/image/src/avif.rs, crates/nvs-ext/tests/image_decode.rs]
  - id: 4
    title: "an animated input decodes every frame"
    status: open
    context: [extensions/image/src/decode.rs, extensions/image/src/lib.rs, docs/rules/core-classes/image-pipeline.md]
  - id: 5
    title: "the four rules are shipped, and the roster says the component is whole"
    status: open
    context: [docs/rules/core-api/tier-roster.md]
checks: []
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
stops naming the gap. It was milestone M17's work, and M17 is gone.

## Standing decisions

**The defaults are the rule's, for every format.** AVIF and JPEG XL take the same two `open` options as
the formats `image` decodes, on by default, and `encode` strips what they carried
(`rule:core-classes/image-correct-by-default`). A profile that does not parse is ignored, as it is for
the others.

**`exif` is read from the header, and a malformed block is a `Parse` error**, as `decode` already
treats one (`extensions/image/src/decode.rs:10`). `info` still decodes no pixel.

**`frames` is counted without decoding a frame.** Each container states its frame count or lists its
frames, so stage 3 reads it from the header and costs no pixel memory.

**Open, and the user's: what decoding every frame means.** `rule:core-classes/image-pixel-cap` prices
one frame and spends at most two frames at the cap per call, and
`rule:core-classes/image-pipeline` has no per-frame member. Stage 4 needs both rules to say what a
pipeline does with the other frames (keep them through every step, or give the program a frame to
pick) and how the cap counts an animation (per frame, or the sum). That is a change to a feature's
surface and its memory bound, so it is asked before stage 4 starts, never decided by a session.

**Trade-offs.** A program gains: a phone photo in AVIF or JPEG XL is upright and in the right colours,
and `info` returns the tags it used to need another library for. Request-path cost is one EXIF parse
per `info` call and one colour transform per AVIF or JPEG XL `open`, both on work the call already
does. Memory for stages 1 to 3 is one extra frame during the sRGB conversion, as the other formats
already pay, per request, freed with it. Stage 4's memory is the open decision above.

## Stages

### 1. an AVIF and a JPEG XL input open upright and in sRGB

**Does:** an AVIF and a JPEG XL fixture with an orientation tag and an embedded profile decode upright
and in sRGB, and each option set to `false` returns the raw frame. A test in
`crates/nvs-ext/tests/image_pipeline.rs` pins each format.

### 2. `Image::info` returns the `exif` fields a file carries

**Does:** `info` on a JPEG, PNG, WebP, AVIF and JPEG XL fixture with an EXIF block returns its fields,
`null` for a file with none, and a `Parse` error for a malformed block.

### 3. `Image::info`'s `frames` is an animated input's real count

**Does:** an animated fixture of each animated format the roster decodes (APNG, GIF, WebP, AVIF and
JPEG XL) gives its frame count, and a still image gives `1`.

### 4. an animated input decodes every frame

**Does:** once the user has decided the open question above and the two rules say it, an animated
fixture of each format decodes every frame, and the cap is held as the rule says.

### 5. the four rules are shipped, and the roster says the component is whole

**Does:** `image-correct-by-default`, `image-format-roster`, `image-pixel-model` and `tier-roster`
lose their *Partly on disk* lines about this work, and each record is `shipped` with its tests in
`guardedBy`. `bun nv proofs --id` prints nothing owed for the members this goal changes.

## Handoff

### State

The goal is new. Nothing in it has started. Stage 4 waits on the user's decision above. The anchors
below were current when it was written.

### Next

Stage 1 and stage 2 together, on the decoders and `info`:
- orient and convert an AVIF frame as the `image` path does (`extensions/image/src/decode.rs:21`, `extensions/image/src/avif.rs`), per `rule:core-classes/image-correct-by-default`
- the same for JPEG XL through `jxl-oxide`'s orientation and ICC (`extensions/image/src/decode.rs:21`)
- read the EXIF block into `Info::exif` (`extensions/image/src/info.rs:25`, `extensions/image/src/info.rs:70`), per `rule:core-classes/image-pixel-model`

### Backlog

- Stage 3's frame count, then ask the user stage 4's question. `docs/rules/core-classes/image-format-roster.md`
- Stage 5 closes the rules. `docs/rules/core-api/tier-roster.md`
