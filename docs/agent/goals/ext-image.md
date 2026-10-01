---
milestone: M9
position: last
---
# Loop goal 195 — `Novis\Image` is built into every binary: it decodes, transforms and encodes inside the sandbox, under the pixel cap

The first of the two components ADR 0247 builds into every `nvs` binary. The image component is one
Rust crate under `extensions/image/`, compiled for `wasm32-wasip2` by a build script, packed into a
`.nvsx` with the packer goal `ext-host` wrote, and embedded. It is loaded before any `[[extension]]`
entry, needs no entry and no pin, and runs every call in the sandbox under the calling request's caps.

```nvs
use Novis\Image\{Image, Fit, Format};

$info  = Image::info($upload);                       // header only, never a pixel
$thumb = Image::open($upload)                        // reads the header, applies the pixel cap
    ->resize({fit: Fit::Cover, width: 800, height: 600})
    ->format(Format::Webp, {quality: 80});
$bytes = $thumb->encode();                           // one call into the component
$set   = $thumb->variants([{width: 400}, {width: 800}]);   // one call, two outputs
```

The design is ADR 0120 (the builder § 2, the entry points § 3, the roster § 5, the cap § 6, the
defaults § 7) as ADR 0247 places it: built in, always present, under `Novis\` rather than in a package.
This goal is the pipeline. Comparison, hashing, placeholders, palette, QR codes and text are goal
`ext-image-analysis`; SVG and PDF decoding are M17's second wave and are not in either.

## Why here

Goals `ext-design`, `ext-host`, `ext-compiler`, `ext-grants` and `ext-tooling` come first and
deliver everything this goal stands on: `wit/image.wit` with § 3's eight exports parsed against the
`nvs:ext@1.0.0` world, `crates/nvs-ext` with its loader, packer, per-request instances, call bridge,
budget charge and failure mapping, the compiler's registration layer and source section, the grants,
and `nvs ext`. None of that is repeated here. What is new is the first component a user calls without
writing a line of configuration, and the build that produces it.

It is the image component first because it is the security claim made legible: an attacker-supplied
file decoded in a sandbox, under the request's memory cap and epoch deadline, with a pixel cap read
from the header before a buffer exists (`rule:core-classes/image-pixel-cap`). And it carries M9's
second-language proof: libwebp, compiled from C to a wasm static library
(`rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci`).

It carries `position: last` because every M9 goal sits behind the pinned closing goals, and it comes
after `ext-tooling` because the build script uses the packer `nvs ext build` uses.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands the
behaviour and not before:

- The **Not shipped** paragraphs of `docs/rules/core-classes/image-pipeline.md`,
  `image-pixel-model.md`, `image-format-roster.md`, `image-pixel-cap.md` and
  `image-correct-by-default.md`. Each becomes what is on disk; `image-one-entry-point-per-job.md`
  is goal `ext-image-analysis`'s.
- The **Not on disk** paragraphs of `docs/rules/packaging/the-first-party-components-are-built-in.md`
  (its image half; the intl half is goal `ext-intl`'s),
  `docs/rules/packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci.md` and
  `docs/rules/security/image-component-declares-nothing.md`.
- `docs/rules/packaging/the-boundary-is-the-cost.md`'s sentence "In-guest compute throughput relative
  to native is not yet measured" — Stage 5 replaces it with the committed figure.
- `crates/nvs-config/src/default.toml:1014` names `/usr/lib/novis/image.nvsx` as its `[[extension]]`
  example. The image component is no longer an entry, so the example names a neutral third-party
  file (`geo.nvsx`), unless goal `ext-host` already changed it.

The search that closes the stage:
`grep -rn "Not shipped\|Not on disk\|not yet measured\|image.nvsx\|nvs/image" docs/rules crates/nvs-config/src/default.toml docs/reference`,
read line by line. Every hit is true as it stands, rewritten, or another goal's (the intl half, the
analysis entry points, M17's PDF and SVG). `docs/novis.md`, `docs/ground-rules.md`, the
`docs/rules/*.md` chapters and `website/` are generated and are regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. Every check goals `ext-host`,
`ext-compiler` and `ext-grants` turned green stays green; a built-in component is loaded beside the
`[[extension]]` set and changes nothing about how an entry loads.

## Stage 2 — the build, the keystone

**Does:** Builds `extensions/image` for `wasm32-wasip2` from a build script, embeds it in every binary,
and makes `Novis\Image\Image::info` answer with no `nvs.toml`.

Three file sets, in this order.

**The crate and its embedding:** `extensions/image/Cargo.toml`, `extensions/image/src/lib.rs`,
`Cargo.toml:26` (the workspace's `exclude`), `rust-toolchain.toml:6` (its `targets`),
`crates/nvs-ext/build.rs`, `crates/nvs-ext/src/builtin.rs`, `crates/nvs-config/src/cache.rs:170`.

- **The crate** (`rule:packaging/the-first-party-components-are-built-in`). `extensions/image/` is
  a library crate outside the workspace: `Cargo.toml:26`'s `exclude` gains `"extensions"`, so the
  ordinary `cargo build` never resolves its dependencies for the host target. Its codec core is plain
  Rust that also compiles for the host (Stage 5's native leg needs it); the `wit-bindgen` export glue
  for `wit/image.wit`'s world is under `cfg(target_family = "wasm")`. It is built with `simd128`
  enabled (ADR 0120 § 5). Its first export is `info`.
- **The target.** `rust-toolchain.toml:6`'s `targets` gains `"wasm32-wasip2"`, so rustup installs it
  for every contributor and every CI leg with no step written by hand.
- **The build script** compiles `extensions/image` with `cargo build --release --target
  wasm32-wasip2` into its own target directory (`target/ext/`), packs the module with the packer,
  and writes the `.nvsx` and its sha256 to `OUT_DIR`. It prints `rerun-if-changed` for
  `extensions/image/` and `wit/`, so an incremental build reuses everything. Where the script lives is
  § *Standing decisions*.
- **The built-in set** (`crates/nvs-ext/src/builtin.rs`): the embedded bytes and their digest, loaded
  before any `[[extension]]` entry, with no entry and no pin. Every load check of goal `ext-host`
  still runs on them, so a component that fails its own manifest check fails the build's test, never a
  user's boot. Compiled on first use in a process and stored in the artifact cache like any other
  module (`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`); a process that never calls
  it compiles nothing.
- **The binary is the pin.** The built-in digest enters `env_hash` at
  `crates/nvs-config/src/cache.rs:170` (`env_hash_of`) beside the build stamp, so two binaries
  carrying different component bytes key apart. `nvs-config` does not depend on `nvs-ext`, so the
  digest is passed in, the way `build` is passed to `env_hash_of` today; the seam is the session's
  call.
- **`Novis\` is refused for everyone else.** Goal `ext-host` refuses a `[[extension]]` declaring a
  `Novis\` class. This goal proves it with the built-in set loaded: an entry declaring
  `Novis\Image\Codec` is refused as reserved, not as a duplicate.

**libwebp:** `tools/nv/cmd/webp-lib.ts` (new, registered in `tools/nv/main.ts`'s command table beside
`webcrypto-vectors` at line 72), `extensions/image/libwebp/` (new), `extensions/image/build.rs` (new),
`.github/workflows/ci.yml:252` (the `extension-sandbox` job, which the new job sits beside),
`tools/nv/cmd/ci-changes.ts:41` (`LANES`).

- **`bun nv webp-lib`** (`rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci`) downloads the
  pinned libwebp release, checks its sha256, compiles the encoder with wasi-sdk for
  `wasm32-wasip2` with `-ffile-prefix-map` so the bytes do not name the machine, and writes
  `extensions/image/libwebp/libwebp.a` beside `extensions/image/libwebp/SOURCE.json` (the version,
  the source tarball's sha256, the wasi-sdk version, the flags, the library's sha256).
  `--check` reads only the committed files and prints
  `libwebp: the committed library matches its recorded digest`, so it runs on a machine with no
  wasi-sdk. `--rebuild` compiles from source and fails when the bytes differ; CI runs it.
- **The link.** `extensions/image/build.rs` links the committed `libwebp.a` (`rustc-link-search` and
  `static=webp`) for the wasm target only, and the crate calls `WebPEncodeRGBA` through a few lines of
  its own `extern "C"`. No `cc` and no `-sys` crate, so no contributor needs a C toolchain.
- **The CI job** `webp-lib`, on Linux, installs the pinned wasi-sdk and runs `bun nv webp-lib
  --rebuild`. A lane in `tools/nv/cmd/ci-changes.ts:41` turns it on for a change under
  `extensions/image/libwebp/` or to the tool.

**The licences:** `tools/nv/cmd/gen-attribution.ts:216` (its `cargo metadata` call),
`tools/nv/cmd/gen-attribution.ts:125` (`C_DEPENDENCIES`), `deny.toml`, `.github/workflows/ci.yml:462`
(`supply-chain`).

- **The notice carries the component.** `gen-attribution` reads `cargo metadata` for
  `extensions/image/Cargo.toml` as well as the workspace, so every crate the component links is in
  `THIRD-PARTY-LICENSES.txt`, and libwebp's BSD-3 text is read from its tarball.
- **libwebp is recorded** in `C_DEPENDENCIES` with the verdict `no-native-code`, the reason naming
  that it runs only inside the image component; `--check-c-deps` prints its line.
- **`cargo deny`** runs over `extensions/image/Cargo.toml` in the `supply-chain` job too.
- **Pinned by** the Stage 2 checks.

## Stage 3 — decode, and the cap

**Does:** Decodes every first-wave format inside the guest, reads `info` from the header, and refuses an
image over `[image] max_pixels` before a buffer is allocated.

Two file sets. The configuration: `crates/nvs-config/src/tree.rs:75` (`Config`),
`crates/nvs-config/src/directive.rs:108` (the `limits.max_decompressed` row, the model),
`crates/nvs-config/src/default.toml:62`. The guest: `extensions/image/src/decode.rs` (new),
`extensions/image/fixtures/` (new), `crates/nvs-ext/tests/image_decode.rs` (new).

- **`[image] max_pixels`** (`rule:core-classes/image-pixel-cap`, ADR 0120 § 6), default `"24M"`, a
  `System`/`Reload` block like `limits.max_decompressed` (`directive.rs:108`), argued where it is
  added (`rule:config/every-block-is-argued-where-it-is-added`) and transcribed into `default.toml`.
  The guest reads it through `nvs:ext/settings`; a built-in component's settings block is the
  top-level `[image]`, not `[ext.image]` (ADR 0247 § 2).
- **The roster** (`rule:core-classes/image-format-roster`): JPEG (`zune-jpeg`), PNG and APNG
  (`png`), WebP lossy and lossless (`image-webp`), GIF with all frames (`gif`), AVIF (`rav1d`) and
  JPEG XL decode only (`jxl-oxide`), under the `image` crate as the container. Each has a small
  fixture under `extensions/image/fixtures/`, made here and never downloaded.
- **`info`** reads the header and never a pixel: format, width, height, alpha, frames, orientation,
  the EXIF shape (`kamadak-exif`), whether an ICC profile is present.
- **The cap before allocation.** Width × height × frames is read from the header and compared with
  the lower of the setting and the call's `{maxPixels}`; over it, the call returns `err(runtime(…))`
  naming the cap and the declared size, which throws `RuntimeError`. A `{maxPixels}` above the
  setting does not raise it. The test reads the instance's peak linear memory and asserts it stayed
  below one frame of the declared size.
- **Malformed input.** Bytes no roster codec recognises return `err(parse(…))`, a `ParseError`. A
  malformed EXIF IFD is a `ParseError` raised inside the guest, never a trap: the IFD reader's errors
  are mapped, not unwrapped. Any other codec fault is a trap, which goal `ext-host` already turns
  into `ExtensionError`.
- **Pinned by** the Stage 3 checks.

## Stage 4 — the pipeline

**Does:** Writes the `Novis\Image` builder in Novis over `run` and `variants`, with auto-orientation,
ICC to sRGB and metadata stripping as the defaults, and `variants` costing one crossing.

Two file sets. The builder: `extensions/image/nvs/` (new: `Image.nvs`, `Color.nvs` and one file per
enum — `Fit`, `Format`, `Gravity`, `Filter`, `Axis`, `Blend`), carried in the `.nvsx`'s
`nvs.source` section. The guest: `extensions/image/src/plan.rs` (new),
`extensions/image/src/encode.rs` (new), `crates/nvs-ext/tests/image_pipeline.rs` (new).

- **The builder** (`rule:core-classes/image-pipeline`, ADR 0120 § 2's table): `open`, `create`,
  `info`, `resize`, `crop`, `trim`, `rotate`, `flip`, `composite`, `flatten`, the seven adjustments,
  `format`, `metadata`, `encode`, `variants`, `raw` and `fromRaw`, each with the signature the table
  gives. An `Image` is an immutable value of the source and a plan; every operation returns a new value
  sharing the bytes. `Format` carries `mime()` and `extension()`. `text`, `compare`, `hash`,
  `hashDistance`, `placeholder` and `palette` are goal `ext-image-analysis`'s.
- **`raw` and `fromRaw`** run where `wit/image.wit`'s test list places them (goal `ext-design`
  decided it, and wrote a record if it had to); this goal implements what that list says.
- **The plan runs in the order written** (`extensions/image/src/plan.rs`), fused only where the
  result differs by resampling rounding alone. Resampling is `fast_image_resize`, `Lanczos3` by
  default.
- **Correct by default** (`rule:core-classes/image-correct-by-default`): `open` applies the EXIF
  orientation and converts an embedded ICC profile to sRGB (`moxcms`); `encode` writes sRGB, embeds
  no profile, and strips EXIF, XMP, IPTC and ICC unless the plan carries `metadata({keep: true})`.
- **The encoders** (ADR 0120 § 5): `jpeg-encoder` with `progressive`, `png`, WebP lossless through
  `image-webp` and lossy through the linked libwebp, `gif` quantised with `color_quant`, AVIF with
  `ravif`.
- **One crossing per terminal.** `encode` and `raw` are one `run` call; `variants` is one `variants`
  call however many entries it has. The test counts host-to-guest calls through nvs-ext's per-instance
  call counter; if goal `ext-host` left none, this item adds one.
- **Pinned by** the Stage 4 checks, which are ADR 0120 § *Verification*'s fixtures one by one.

## Stage 5 — the benchmark

**Does:** Measures a resize and a JPEG decode in the guest against the same crates built native, and
commits the numbers.

One file set: `crates/nvs-ext/benches/image_guest.rs` (new), `crates/nvs-ext/tests/image_guest.rs`
(new), `benches/results/image-guest.json` (new), `docs/rules/packaging/the-boundary-is-the-cost.md`.

- **Two arms per job**, the guest through nvs-ext's call bridge and the native through
  `extensions/image`'s codec core as a host-built dev-dependency, over the same fixture and plan.
  The test first asserts the two outputs are identical byte for byte, so the bench compares the same
  work.
- **The numbers are committed** in `benches/results/image-guest.json`, taken from a release build:
  the median of each arm and the guest-to-native ratio. A test reads the file and fails when an arm
  is missing.
- **The rule states the figure.** `rule:packaging/the-boundary-is-the-cost` replaces "not yet
  measured" with the measured ratio and names the results file. If the ratio is far below native for
  a real workload, ADR 0120's *Revisiting* names the exit; the session records the numbers and puts
  the question in the handoff's `## Backlog` for the user.

## Stage 6 — the feature proofs

**Does:** Teaches the proofs tooling to see a `Novis\` class, then writes the feature proofs for every
`Novis\Image` member this goal built.

Two file sets, in this order. The tooling: `crates/nvs-cli/src/meta.rs:173`, `tools/nv/proofs/roster.ts:287`,
`crates/nvs-cli/tests/meta.rs:10`, `tools/nv/cmd/reference.ts`. The proofs: `docs/examples/novis/`,
`tests/hostile/novis/`, `benches/members/novis/`, `docs/reference/novis/Image.md` (new).

- **The roster sees `Novis\`.** This is the stage's first item, because the proofs tooling does not
  know a `Novis\` class today: `nvs meta --json` (`crates/nvs-cli/src/meta.rs:173`) lists `CLASSES`
  and `ENUMS` from the `Core` registry only, and `roster.ts:287` gives every class the path
  `core/<tail>/<member>`. `nvs meta --json` gains the built-in components' classes and enums, read
  from their manifests and source halves, each member's card from the manifest's help text or the
  source's doc comment. The roster gives a `Novis\` class the path `novis/<tail>/<member>`, where
  `<tail>` is the name after `Novis\` with `\` written `-` (`novis/Image-Image/open`), and an enum
  `types/<tail>` as today.
- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench, one attack and its help in the binary. `bun nv proofs --id
  'Novis\Image\Image::resize'` prints what is still owed and the path of each.
- **`Novis\Image\Codec`** owes its tests, its attack and its help in full: an attack that sends
  hostile bytes and a hostile plan straight to the export, past the builder, is the most direct attack
  there is. Its examples and its bench are recorded in `data/proofs/policy.json`'s `skip`, one reason
  each, naming the builder member that carries them.
- **The attacks** under `tests/hostile/novis/`: a decompression bomb, a header that lies about its
  size, a truncated frame, a malformed IFD, a plan asking for a canvas over the cap, an
  orientation tag out of range. Each ends where `tests/hostile/README.md`'s contract says.
- **The reference page** `docs/reference/novis/Image.md`, in the shape of
  `docs/reference/core/Path.md`. If `bun nv reference` does not read `docs/reference/novis/`, this
  item teaches `tools/nv/cmd/reference.ts` to.
- **Pinned by** the Stage 6 checks. Their `--only` list is ADR 0120 § 2's names; if the roster
  spells one differently, the check's list is corrected to the roster's spelling and the commit says
  why.

## Standing decisions

- **The user's calls, as instructions.** Values cross as typed WIT values by ADR 0246 § 1's table,
  and the `value` handle only for `mixed`. A guest links WASI with an empty context and may be granted
  files and outbound HTTP and nothing else; the image component is granted nothing. Grants are the
  intersection of the entry's, the manifest's and the caller's. A guest call runs on its request's
  core as a wasmtime async call polled by the coroutine, yielding at every epoch tick, with no compute
  pool. The Novis source half travels inside the `.nvsx` in `nvs.source`. A trap throws
  `ExtensionError`; a CPU or memory limit is a resource-limit `FATAL`. `Novis\Image` and `Novis\Intl`
  are built into every binary and always on: no setting and no Cargo feature turns them off. The build
  compiles the component crates for `wasm32-wasip2`; libwebp is prebuilt by a `bun nv` tool and
  committed beside its source hash, and CI rebuilds and compares. `sha256` is required on every
  `[[extension]]` entry, and a built-in component has none because the binary is its pin. No
  signatures in M9.
- **The record writer's calls, not confirmed by the user, also standing.** The manifest is JSON from
  an `nvsx.toml`; `Novis\` is reserved; the error variant `invalid|parse|runtime` throws
  `LogicError|ParseError|RuntimeError`; one instance per extension per request, a second task waits;
  `nvs check` and the LSP read manifests and never instantiate; a bundle embeds the `.nvsx` files its
  configuration lists, and the built-in components are in every bundle because they are in the
  binary.
- **The crates are ADR 0120 § 5's**, and no session replaces one: `image` as the container,
  `zune-jpeg` and `jpeg-encoder`, `png`, `image-webp` and libwebp, `gif` and `color_quant`, `rav1d`
  and `ravif`, `jxl-oxide`, and `fast_image_resize`. **The goal writer's picks, unconfirmed:**
  `moxcms` for ICC to sRGB (fallback `qcms`), `kamadak-exif` for the EXIF shape. Every crate passes
  `cargo deny` against `deny.toml`. Whether each builds for `wasm32-wasip2` is not checked; if `rav1d`
  does not build there with its assembly off, the AVIF decode item stops and the question goes in the
  handoff for the user, and every other format continues.
- **Where the build script lives** is the goal writer's call, unconfirmed: `crates/nvs-ext/build.rs`,
  as the plan names it. A build script cannot call its own crate, so it compiles the packer's source
  file into itself with `#[path]`, and `nvs ext build` and the build run the same code. If the packer
  reaches crate-internal types and cannot be compiled alone, the fallback is a small crate
  `crates/nvs-builtin` whose build script takes `nvs-ext` as a build-dependency; the Stage 2 checks
  then name that crate, and the commit says so.
- **The image fixtures are made here.** Every file under `extensions/image/fixtures/` is produced
  by a generator in the crate's tests or by hand from a free tool, never downloaded from a photo
  site, and carries no personal data. The CMYK fixture's profile must allow redistribution; which
  profile is the session's call, and it is named in the fixture folder's `README.md`.
- **`libwebp.a` is built on Linux.** CI's Linux leg is the reference. On this machine the tool runs
  under WSL (`wsl.exe`) when it compiles; if a Windows and a Linux build ever differ, that is recorded
  in `SOURCE.json` and the Linux bytes are committed.
- **The `[image]` block reads the cap only.** Nothing in it grants anything (ADR 0246 § 9). A key
  beyond `max_pixels` is a new decision and goes in the handoff's `## Backlog`.
- **No record slot.** ADR 0120 and ADR 0247 decide this goal, and goal `ext-design`'s record, if it
  wrote one, is already on disk. A session that meets a question those records do not answer decides
  it under AGENTS.md's priority ordering and writes the answer into the rule fragment it changes.
- **The tradeoffs**, stated here because AGENTS.md asks. Performance: one host-to-guest call per
  terminal, the pixel buffer never leaves the guest, and a program that never calls the component
  compiles nothing; in-guest compute is slower than native by the factor Stage 5 measures. Memory: up
  to two frames at the cap per calling request (192 MB at 24 MP), charged to that request's memory cap
  and freed when it ends; nothing at all for a request that does not call it. Usability: image
  handling works on every install, in every bundle and in tests, with no configuration, and the
  output is upright, colour-correct and free of location data by default. Simplicity: one more build
  step, one committed binary library with a check that it matches its source, and a builder surface of
  one member per job with no mode flags.
- **Neutral names only** in every fixture, test, example and record — `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ext` or a
  `--test` filter.
