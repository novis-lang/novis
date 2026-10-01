---
milestone: M9
position: last
---
# Loop goal 196 — `Novis\Image` compares, hashes, summarises and draws: comparison, perceptual hashes, placeholders, palette, QR codes and text

The rest of ADR 0120's image component, over the crate, the builder and the build goal `ext-image`
landed. Each job is one entry point (`rule:core-classes/image-one-entry-point-per-job`), and each is one
host-to-guest call carrying its whole input.

```nvs
use Novis\Image\{Image, HashKind, PlaceholderKind, QrCode, Font, Color};

$diff  = Image::compare($expected, $actual);          // identical, differingPixels, maxDelta, ssim
$hash  = Image::hash($upload, HashKind::Perceptual);
$near  = Image::hashDistance($hash, $stored) <= 6;
$blur  = Image::placeholder($upload, PlaceholderKind::BlurHash);
$theme = Image::palette($upload, 5);
$qr    = QrCode::render("https://example.com/t/42", {size: 256});
$card  = Image::create(1200, 630, Color::hex("#1d3557"))
    ->text("Spring sale", Font::fromBytes($fontBytes), {size: 72, color: Color::rgba(255, 255, 255)});
```

ADR 0120 § 8 is comparison, hashing and placeholders; § 9 is text and QR codes. SVG rasterising and
PDF pages are the second wave, M17's, and not this goal's: ADR 0120 *Verification*'s fixture "an SVG
referencing an external file rasterises with that reference unresolved" is M17's too.

## Why here

It needs everything goal `ext-image` built — the crate under `extensions/image/`, its build script
and embedding, the builder source in `extensions/image/nvs/`, the decoders and encoders, the pixel
cap, the crossing counter — and it adds entry points to that same component, so it follows it
directly. Goal `ext-design`'s `wit/image.wit` already carries `compare`, `hash`, `placeholder`,
`palette` and `qr` among its eight exports, and its test list says where `hashDistance` and
`measureText` run; this goal implements what that list says.

These are the members a web application reaches for after the pipeline: a test asking whether two
images are the same, near-duplicate detection for uploads, a blurred placeholder and a theme colour
for the front end, a QR code, and a social card with text. None of them needs a new mechanism, which is
why they are one goal of breadth after the keystone.

It carries `position: last` because every M9 goal sits behind the pinned closing goals.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/core-classes/image-one-entry-point-per-job.md`'s **Not shipped** paragraph. It
  becomes what is on disk, and says that SVG is the second wave's.
- `docs/reference/novis/Image.md` (goal `ext-image` wrote it) gains a section per job, and its text
  section says that complex shaping is not yet in.

The search that closes the stage:
`grep -rn "Not shipped\|compare\|placeholder\|QrCode\|measureText" docs/rules/core-classes/image-*.md docs/reference/novis`,
read line by line. Every hit is true as it stands, rewritten, or M17's. `docs/novis.md`,
`docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and are
regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. Every check goal `ext-image` turned
green stays green; no change here alters what `run` or `variants` returns.

## Stage 2 — comparison

**Does:** Adds `Image::compare`, which decodes two images in one call and reports whether they are
identical, how many pixels differ, the largest difference, the SSIM score and, on request, a diff image.

One file set: `extensions/image/src/compare.rs` (new), `extensions/image/nvs/Image.nvs`,
`extensions/image/nvs/Diff.nvs` (new), `crates/nvs-ext/tests/image_analysis.rs` (new).

- **`compare(bytes|Image $a, bytes|Image $b, {tolerance?: uint, render?: bool}): Diff`**
  (`rule:core-classes/image-one-entry-point-per-job`, ADR 0120 § 8). An `Image` argument is run by
  its own pipeline to PNG first, one crossing each, and the two results go to the `compare` export;
  two `bytes` arguments cost one crossing.
- **`Diff`** is readonly: `identical`, `differingPixels`, `maxDelta`, `ssim`, and `diff`, a PNG with
  the differing pixels marked, only with `{render: true}`. A pixel differs when any channel differs by
  more than `tolerance` (default `0`).
- **SSIM is written in the component.** `dssim` is AGPL and outside `deny.toml`. The window and the
  channel weighting are the session's call and are named in the function's doc; an image compared with
  itself is exactly `1.0`.
- **A size mismatch** returns `err(invalid(…))`, a `LogicError` naming both sizes.
- **No assertion member** (ADR 0120 § 8): a test writes
  `Core\Test::assertTrue(Image::compare($a, $b)->ssim >= 0.99)`.
- **Pinned by** the Stage 2 checks; the first of them is ADR 0120 *Verification*'s compare fixture.
  Their images are built in the test with `create` and `encode`, so no fixture file is needed.

## Stage 3 — hashes, placeholders, palette

**Does:** Adds `Image::hash`, `Image::hashDistance`, `Image::placeholder` and `Image::palette`, each one
job in one call.

One file set: `extensions/image/src/hash.rs`, `extensions/image/src/placeholder.rs`,
`extensions/image/src/palette.rs` (all new), `extensions/image/nvs/Image.nvs`,
`extensions/image/nvs/HashKind.nvs`, `extensions/image/nvs/PlaceholderKind.nvs` (new),
`crates/nvs-ext/tests/image_analysis.rs`.

- **`hash(bytes $data, HashKind $kind): bytes`** with `Perceptual`, `Difference` and `Average`.
  Each kind returns a hash of a fixed length, named in the reference page.
- **`hashDistance(bytes $a, bytes $b): uint`** is the Hamming distance, and runs where
  `wit/image.wit`'s test list places it. Two hashes of different lengths are a `LogicError`.
- **`placeholder(bytes $data, PlaceholderKind $kind): string`** with `BlurHash` and `ThumbHash`. The
  image is scaled down inside the guest first, so a large upload costs one small decode.
- **`palette(bytes $data, uint $count = 5): array<Color>`** returns at most `$count` colours, most
  frequent first, quantised with `color_quant`, which the GIF encoder already links.
- **Pinned by** the Stage 3 checks.

## Stage 4 — QR codes

**Does:** Adds `Novis\Image\QrCode::render`, which encodes a string as a QR code image in one call.

One file set: `extensions/image/src/qr.rs` (new), `extensions/image/nvs/QrCode.nvs`,
`extensions/image/nvs/QrLevel.nvs` (new), `crates/nvs-ext/tests/image_analysis.rs`.

- **`QrCode::render(string $data, {size?: uint, margin?: uint, level?: QrLevel, format?: Format}):
  bytes`** (ADR 0120 § 9), over the `qr` export. `QrLevel` is `Low`, `Medium`, `Quartile`, `High`;
  the default level is `Medium`, the default format PNG.
- **Data too long** for the chosen level is a `LogicError` naming the level and the length.
- **The test reads the code back** with `rqrr`, a dev-dependency of `nvs-ext` built native, so a
  code that only looks right fails.
- **Pinned by** the Stage 4 checks.

## Stage 5 — text

**Does:** Adds `$img->text`, `Image::measureText` and `Novis\Image\Font::fromBytes`, which draw and
measure Latin text in a font the program supplies as bytes.

One file set: `extensions/image/src/text.rs` (new), `extensions/image/nvs/Image.nvs`,
`extensions/image/nvs/Font.nvs`, `extensions/image/nvs/Align.nvs` (new),
`extensions/image/fixtures/font/` (new), `crates/nvs-ext/tests/image_analysis.rs`.

- **`text(string $text, Font $font, {size: float, color: Color, gravity?: Gravity, x?: int, y?: int,
  maxWidth?: uint, align?: Align}): Image`** is a plan step (ADR 0120 § 2), so drawing text adds no
  crossing to the terminal that runs it.
- **`Font::fromBytes(bytes)`** checks the font's header in Novis and throws `ParseError` on bytes that
  are not a TrueType or OpenType font; a font whose tables are malformed throws `ParseError` at the
  terminal. No system font is ever looked up: the guest has no filesystem.
- **`Image::measureText(...)`** returns the box `text` would draw, and runs where `wit/image.wit`'s
  test list places it.
- **Latin shaping**: kerning and line breaking at word boundaries for `maxWidth`. Complex shaping
  (`rustybuzz`) is not in this goal, and the reference page says so in one sentence.
- **The fixture font** is a Latin subset of a font under the SIL Open Font License, committed with
  its licence text under `extensions/image/fixtures/font/`.
- **Pinned by** the Stage 5 checks.

## Stage 6 — the feature proofs

**Does:** Writes the feature proofs for every `Novis\Image` member this goal added, which completes
the `Novis\Image` classes.

One file set: `docs/examples/novis/`, `tests/hostile/novis/`, `benches/members/novis/`,
`docs/reference/novis/Image.md`, `data/proofs/policy.json`.

- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench, one attack and its help in the binary. Goal `ext-image` taught the
  roster to see `Novis\`; `bun nv proofs --id 'Novis\Image\Image::compare'` prints what is owed.
- **`Novis\Image\Codec`'s new members** (`compare`, `hash`, `placeholder`, `palette`, `qr`, and
  `measureText` if the list made it an export) follow goal `ext-image`'s rule: tests, attack and help
  in full; examples and bench recorded in `data/proofs/policy.json`'s `skip`, one reason each.
- **The attacks**: two images of different sizes, a hash of the wrong length, a palette count of
  `0` and a huge one, a QR payload past the largest version, a font file that lies about its tables,
  text far wider than the canvas.
- **Pinned by** the Stage 6 checks. Their `--group` and `--only` lists are ADR 0120's names; if the
  roster spells one differently, the list is corrected to the roster's spelling and the commit says
  why.

## Standing decisions

- **The user's calls, as instructions.** Values cross as typed WIT values by ADR 0246 § 1's table,
  and the `value` handle only for `mixed`. A guest links WASI with an empty context and may be granted
  files and outbound HTTP and nothing else; the image component is granted nothing. Grants are the
  intersection of the entry's, the manifest's and the caller's. A guest call runs on its request's
  core as a wasmtime async call polled by the coroutine, yielding at every epoch tick, with no compute
  pool. The Novis source half travels inside the `.nvsx`. A trap throws `ExtensionError`; a CPU or
  memory limit is a resource-limit `FATAL`. `Novis\Image` and `Novis\Intl` are built into every
  binary and always on. The component crates are compiled for `wasm32-wasip2`, and libwebp is
  prebuilt and checked in CI. No signatures in M9.
- **The record writer's calls, not confirmed by the user, also standing.** The error variant
  `invalid|parse|runtime` throws `LogicError|ParseError|RuntimeError`; one instance per extension per
  request, a second task waits; `nvs check` and the LSP read manifests and never instantiate.
- **The crates.** ADR 0120 names `color_quant` (already linked) and rules out `dssim`. **The goal
  writer's picks, unconfirmed:** `image_hasher` for the three hashes (fallback: the three written in
  the component over `rustdct`), `blurhash` and `thumbhash` for the placeholders, `qrcode` for QR
  encoding and `rqrr` to read it back in tests only, and `ab_glyph` for glyph outlines and kerning.
  Each passes `cargo deny` against `deny.toml` and lands in the third-party notice through goal
  `ext-image`'s attribution reader. A crate that does not build for `wasm32-wasip2` is replaced by
  its fallback or written in the component; this never needs the user.
- **Where `hashDistance` and `measureText` run** is goal `ext-design`'s answer in `wit/image.wit`'s
  test list, and its record if it wrote one. No session re-decides it here.
- **`compare` with an `Image` argument** runs that pipeline first, one crossing per `Image`. The
  goal writer's call, unconfirmed: it keeps `compare`'s export at two `bytes` as ADR 0120 § 3 states.
- **`Font::fromBytes` checks the header in Novis**, with no crossing. The goal writer's call,
  unconfirmed: a font's tables are parsed by the terminal that uses it.
- **No record slot.** ADR 0120 decides this goal. A question it does not answer is decided under
  AGENTS.md's priority ordering and written into the rule fragment it changes.
- **The tradeoffs**, stated here because AGENTS.md asks. Performance: each job is one crossing with
  its whole input; `compare` on two `Image` values is three. Memory: at most two decoded frames at
  the pixel cap per call, charged to the calling request and freed when it ends; nothing for a
  request that does not call them. Usability: a test compares images by their numbers, uploads are
  checked for near-duplicates, and the front end gets placeholders, a palette, QR codes and cards
  with no other library. Simplicity: one member per job, no drawing primitives, and Latin text only
  until complex shaping lands.
- **Neutral names only** in every fixture, test, example and record — `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ext` or a
  `--test` filter.
