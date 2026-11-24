---
looper: 1
title: "a program writes zip archives as a stream, and reads one entry as a stream"
summary: "Reached when Core\\Zip::createForFile and ::createForDownload return a Core\\Zip\\Writer whose add writes each entry as it is read at a deflate level from 0 to 9 and whose finish writes the directory, every name the reader would refuse throws before a byte is written, an archive past 4 GiB or 65535 entries is written with zip64, Core\\Zip::stream gives one entry as chunks under the decompression bound, extract holds one block and never a whole entry, Core\\Compress::compress and ::compressor take a level in each codec's own range, the spreadsheet writer frames its zip with the same code, and every new or changed member has its proofs, help and pages."
context: [crates/nvs-stdlib/src/zip.rs, docs/rules/core-classes/decompression-bound.md, docs/rules/core-api/tier-roster.md]
stages:
  - id: 1
    title: the rules say how a zip is written and read as a stream, and what a compression level is
    status: open
    context: [docs/rules/core-classes/spreadsheet-writes-as-a-stream.md, docs/rules/core-api/a-member-takes-the-source-the-program-holds.md, docs/agent/doc-style.md, crates/nvs-stdlib/src/compress.rs]
  - id: 2
    title: "Core\\Compress::compress and ::compressor take a level"
    status: open
    context: [crates/nvs-stdlib/src/compress.rs, Cargo.toml]
  - id: 3
    title: one crate frames a zip, and the spreadsheet writer uses it
    status: open
    context: [extensions/spreadsheet/src/zip.rs, extensions/spreadsheet/Cargo.toml, extensions/spreadsheet/src/lib.rs, Cargo.toml]
  - id: 4
    title: "Core\\Zip::createForFile, Writer::add and Writer::finish write an archive to a file"
    status: open
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/byte_source.rs, crates/nvs-stdlib/src/io.rs, extensions/spreadsheet/nvs/Workbook.nvs]
  - id: 5
    title: "Core\\Zip::createForDownload streams an archive to the browser"
    status: open
    context: [crates/nvs-stdlib/src/response.rs, extensions/spreadsheet/nvs/Workbook.nvs]
  - id: 6
    title: "extract holds one block, and Core\\Zip::stream gives one entry as chunks"
    status: open
    context: [crates/nvs-stdlib/src/byte_source.rs, crates/nvs-stdlib/src/http.rs]
  - id: 7
    title: the feature proofs of every new or changed member
    status: open
    context: [docs/agent/testing.md, docs/agent/prose.md]
  - id: 8
    title: "the help, the reference and the website say what Core\\Zip writes and what a level does"
    status: open
    context: [docs/agent/prose.md, website/src/data/changes.json]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: compress-level
    stage: 2
    name: "Core\\Compress::compress and ::compressor write at a level in each codec's range, and a level outside it throws naming the range"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/compress-a-level-is-the-codec-s-own-number.nvst]
    timeout: 1800
  - id: writer-round-trip
    stage: 4
    name: "an archive Core\\Zip\\Writer writes to a file lists and reads back through Core\\Zip, at level 0, 1 and 9"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/zip-writer-an-archive-written-to-a-file-reads-back.nvst]
    timeout: 1800
  - id: writer-names
    stage: 4
    name: "every name the reader refuses throws when it is added, before a byte of the entry is written"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/zip-writer-a-name-the-reader-refuses-throws-when-it-is-added.nvst]
    timeout: 1800
  - id: download
    stage: 5
    name: "a download that is never finished is cut off, and the log names it"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/zip-writer-a-download-never-finished-is-cut-off.nvst]
    timeout: 1800
  - id: proofs
    stage: 7
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, zip-writer]
    timeout: 600
  - id: close
    stage: 9
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, zip-writer]
    timeout: 900
---

# a program writes zip archives as a stream, and reads one entry as a stream

## Why

`Core\Zip` reads archives and cannot write one. Its three members, `entries`, `read` and `extract`,
all read (`crates/nvs-stdlib/src/zip.rs:186`). No rule declines writing: the module doc and
`rule:core-api/tier-roster` argue only why *reading* is Tier 0. Three places in the tree write a zip
by hand, so the gap is already paid for three times:

- the spreadsheet component frames its xlsx zip itself (`extensions/spreadsheet/src/zip.rs:1`);
- `nvs pkg pack` writes package archives in Rust;
- the Zip benches assemble archives byte by byte with `Core\Bytes::pack`
  (`benches/members/core/Zip/entries.scale.nvs:10`), and the examples embed base64.

A program today has two ways to make a zip: the same `Core\Bytes::pack` assembly, or the host's `zip`
binary through `Core\Process::run`. Neither is acceptable for a web server's "download these files"
or a CLI's backup.

`Core\Compress` has the same hole one level down: `compress` and `compressor` take a codec and no
level (`crates/nvs-stdlib/src/compress.rs:253`, `:279`). Each codec is fixed in `encode`
(`compress.rs:889`): deflate's default 6 for gzip, zlib and deflate, brotli quality 5, and zstd
`Fastest`. A program that compresses once and serves many times cannot ask for the smaller result,
and one on a hot path cannot ask for the faster one.

## Standing decisions

**Writing is a stream, and only a stream (the user's call: memory is never the issue).** A
`Core\Zip\Writer` writes each entry as it reads it, and never holds an entry or the archive. Entries
are framed with a data descriptor (general-purpose bit 3), so the writer never seeks back, exactly as
`rule:core-classes/spreadsheet-writes-as-a-stream` frames xlsx. There are two targets:

- `Core\Zip::createForFile(#[Core\Path] string $path, uint $level = 6): Core\Zip\Writer` writes to
  `<path>.<16 hex>.part` and renames it to `$path` in `finish()`. A failed write deletes the part
  file. This is `Workbook::createForFile`'s shape (`extensions/spreadsheet/nvs/Workbook.nvs:83`).
- `Core\Zip::createForDownload(string $name, uint $level = 6): Core\Zip\Writer` sets
  `Content-Disposition` and opens `Core\Response::startStream('application/zip', null, null, $name)`, so
  the headers leave at once and each entry leaves as it is compressed. A download the program never
  finishes is cut off and logged, through `$mustEnd`, as `Workbook::createForDownload` is
  (`Workbook.nvs:70`).

There is no in-memory target and no `createForStream`. A program that needs the archive's bytes, for
a mail attachment or `Core\Storage::put`, writes it to a file under `Core\IO::temporaryDir()` and
passes the file. That costs one temporary file and keeps the writer's memory independent of the
archive's size, which is the user's ask.

**The writer's members are three, and nothing more.**

- `add(string $name, Core\Bytes::Source|Core\Bytes::Chunks $content, ?uint $level = null): void`
  writes one entry from any source `rule:core-api/a-member-takes-the-source-the-program-holds` names.
  A `Core\Bytes::Chunks` is read once, forward, as `Core\IO::writeStream` reads it. `$level` replaces
  the writer's level for this entry, such as `0` for a JPEG that does not compress.
- `finish(): uint` writes the central directory and the end record, ends the target, and returns the
  archive's size in bytes. A second `finish()`, or an `add` after it, throws `LogicError`.
- There is no folder entry. A folder exists in the archive because a file's name contains it.

**A compression level is the codec's own number, and one rule says so for both classes (the user's
call: a level for zip, and the same follow-up for `Core\Compress`).** A programmer already knows
these numbers from `gzip -9`, `brotli -q 11` and `zstd -1`, so the level is that number and not a
Novis scale mapped onto each codec. The rule is
`docs/rules/core-classes/a-compression-level-is-the-codec-s-own-number.md`, and `Core\Zip`'s level is
deflate's range from it.

| Codec | Range | Default (`null`) | Today's behaviour |
|---|---|---|---|
| `Gzip`, `Zlib`, `Deflate` | `0` to `9` | `6` | `flate2::Compression::default()`, which is 6 |
| `Brotli` | `0` to `11` | `5` | quality 5 with a 22-bit window, and the window stays 22 at every level |
| `Zstd` | `1` | `1` | `ruzstd`'s `Fastest` |

- `Core\Compress::compress(..., Codec $codec, ?uint $level = null)` and
  `Core\Compress::compressor(Codec $codec, ?uint $level = null)` take it last, so every existing
  call compiles and writes the same bytes. The compressor keeps it in one more slot.
- A level outside its codec's range throws `RuntimeError` naming the codec and the range. Clamping
  would give a program a level it did not ask for.
- **Zstd takes only `1`.** `ruzstd` 0.9.1 is the newest release, and its encoder implements
  `Uncompressed` and `Fastest` only. `Default`, `Better` and `Best` are marked unimplemented. The C
  `zstd` library stays declined for the security reason `Cargo.toml:1082` gives. When `ruzstd`
  implements more levels, the range grows, and no call changes. `0` is not `Uncompressed`: in the
  reference tool `0` means the default level, and the same number must not mean the opposite here.
- **What it costs.** A higher level is slower: brotli 11 is about ten times slower to encode than 5,
  for a few percent of size. That is the program's choice per call. Compression already polls the
  request's deadline (`compress_polled`), so a slow level ends at the deadline like any other.
  Memory: brotli's window stays 22 bits at every level. Stage 2 measures the encoder's peak at
  levels 5 and 11, and at deflate 1 and 9, and writes the figures into the rule.

**A zip's level `0` writes:**

- **method 0 (stored), sizes in the local header**, when `$content` is `bytes`. The size and CRC-32
  are known before the entry is written, and an EPUB's or ODF's `mimetype` entry needs exactly this.
- **method 8 with stored deflate blocks and a data descriptor** for every other source. A stored entry
  with a data descriptor is legal, but a forward-only reader such as Java's `ZipInputStream` cannot
  find its end. Stored deflate blocks cost 5 bytes per 64 KiB and every reader takes them.

Levels `1` to `9` write deflate (method 8) through `miniz_oxide`'s compressor at that level. Only
methods 0 and 8 are written. Declined: zstd (93), bzip2 (12) and LZMA (14). Windows Explorer and
macOS Archive Utility do not all open them, and `Core\Zip` itself does not read them.

**A name is judged by the reader's own predicate, before a byte of the entry is written.** The writer
calls the function the reader judges with (`refuse_hostile`, `zip.rs:850`), factored so both call it:
no empty name, no absolute path or drive, no `..`, at most `MAX_FOLDERS` folders. The writer adds three
checks the reader cannot make on its own: a name already added, a name containing `\` (use `/`), and a
name ending in `/` (there are no folder entries). Each throws `RuntimeError`, the class
`Core\Storage::put` throws for a bad key, with a message naming the entry. Every archive Novis writes is
therefore one Novis reads. A `tainted string` name is accepted, because it is judged. Cost: a set of the
names added so far, per writer, until `finish()`.

**Zip64 is written only where a field does not fit.** An archive whose offsets, sizes and entry count
fit 32 and 16 bits has no zip64 record at all, so every old reader opens it. Past 65535 entries or a
4 GiB offset, the zip64 end record, its locator and a central-directory zip64 extra are written. An
entry whose size is known before it is written (`bytes`, `Core\IO\File`, `Core\Request\Part`) and is
at least `0xFFFF_0000` bytes gets a local zip64 extra and an 8-byte data descriptor, as APPNOTE
4.3.9.2 says. A `Core\Bytes::Chunks` entry has no size in advance: when it passes `0xFFFF_0000` bytes,
`add` throws `RuntimeError` and says to pass a file instead. Tests reach these limits through a
test-only threshold in the framing crate, never by writing 4 GiB.

**Every entry has one time: the time the writer was created.** The DOS date field carries it in UTC,
and an extended-timestamp extra field (0x5455) carries the Unix time, so Info-ZIP, 7-Zip and macOS
show the right time in every zone. Declined for now: a per-entry time. Nobody has asked for one yet,
and it is one more parameter on every `add`. Every entry is recorded as made on Unix with mode
`0644`. Declined: a mode or executable bit, and symlinks, which the reader refuses.

**Reading stays random access through the central directory (the user asked whether reading should
be forward-only too: it should not).**

- **Security comes first.** The central directory is the archive's index. A forward-only reader trusts
  each local header instead, and a local header can disagree with the directory. That disagreement is
  how one archive shows a scanner one file and an extractor another. A stored entry with a data
  descriptor cannot even be delimited forward-only. The reader's guarantee that the whole archive is
  judged before any byte is returned or written (`zip.rs:14`) needs the whole directory first.
- **Memory is already bounded.** The reader reads by ranges from every `Core\Bytes::Source` and never
  holds the archive (`zip.rs:99`). What it holds is the directory, one entry per row, and one entry's
  output. An uploaded `Core\Request\Part` is held whole under `[limits] request_body`, and its error
  already names `saveTo` (`rule:http-server/a-part-is-consumed-in-one-of-three-ways`).
- **What this goal streams is one entry's output.**
  `Core\Zip::stream(Core\Bytes::Source $archive, string $name, uint $maxBytes = <ceiling>, uint $maxRatio = <ratio>): Core\Bytes::Chunks`
  gives the entry as chunks of at most 64 KiB, under `rule:core-classes/decompression-bound`, and
  checks the CRC-32 when the last chunk is read. A mismatch throws `ParseError` from that read. It
  feeds `Core\IO::writeStream`, `Core\Storage::put`, `Core\Hash::of` and a response stream without
  holding the entry. `extract` writes each entry through the same path (`Budget::stream_from`,
  `zip.rs:1034`) and holds one block, not a whole entry (`zip.rs:1528` holds it today).
- **`read` stays.** It returns one small entry, such as a `config.json`, under the same bound. Two
  calls for two sizes are simpler than one call that every small read has to join.

**One crate frames a zip.** `crates/nvs-zip-frame` holds the writer's framing: local headers, data
descriptors, the central directory, zip64 and the stored and deflate paths. It depends on
`miniz_oxide` and `crc32fast` only, which both workspaces already have, and builds for the host and
for `wasm32-wasip2`. `nvs-stdlib`'s writer and the spreadsheet component (through a path dependency)
both use it, and `extensions/spreadsheet/src/zip.rs` is deleted. The spreadsheet passes its fixed
1980-01-01 time and keeps zip64 off, so its output bytes and
`rule:core-classes/spreadsheet-output-is-inert` do not change. A change the extension's build needs
for a path dependency outside its workspace is in scope.

**`Content-Disposition` is built in Rust in `response.rs`.** `Workbook.nvs`'s `disposition`
(`Workbook.nvs:345`) is Novis code in a component, and the Rust writer cannot call it. Both follow
RFC 6266 the same way: an ASCII `filename` with `_` for anything else, and a `filename*` in UTF-8. The
Rust function gets a test that compares its output with the Novis one for the same names. Declined: a
new `Core\Response` member for it. That is a change to the `Core` API this goal does not need.

**What the writer costs** (`rule:programs/memory-priority`), per open writer, attributable to its
request: one deflate compressor, reused across entries; one output block; and per entry its name plus
about 40 bytes for the central directory row, until `finish()`. Stage 4 measures the compressor's size
and writes the figure into the writer's doc comment. `Core\Zip::stream` holds one input window and
one output block. Nothing grows with the archive's total size.

**Capabilities.** `createForFile` needs `fs.write` for the path, as `Core\IO::open` in `Write` mode
does. `createForDownload` needs nothing beyond a request. The registry's door table
(`registry.rs:2548`) names both.

**Lanes.** The stages run in order. Stage 2 touches only `compress.rs` and could run beside stage 3,
but both are small, so one session runs them in turn. Stages 3 and 4 share the framing crate, and
stages 4 to 6 share `zip.rs`. Stage 7's proof files are disjoint from each other and go to writing
subagents.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in two
sessions writes what holds it in the handoff and the next session splits it.

## Stages

### 1. the rules say how a zip is written and read as a stream, and what a compression level is

- [ ] `docs/rules/core-classes/zip-is-written-and-read-as-a-stream.md:1`: the rule, added the way
  `docs/agent/doc-style.md` says, from this file's standing decisions: the two targets, the three
  writer members, level `0` and the methods, the name checks, zip64, time and mode, random-access
  reading and `Core\Zip::stream`, the shared framing crate, and what it costs. Its Declined list: an
  in-memory target, `createForStream`, forward-only reading, methods 12, 14 and 93, folder entries, a
  per-entry time, a mode, symlinks, encryption (ZipCrypto is broken, and WinZip AES is a vendor
  extension), and a `Core\Response` member for `Content-Disposition`.
- [ ] `docs/rules/core-classes/a-compression-level-is-the-codec-s-own-number.md:1`: the level table,
  the `null` default per codec, the `RuntimeError` outside a range, zstd's single level and why, and
  what a higher level costs. Its Declined list: a Novis scale such as `Fastest`/`Smallest` mapped onto
  each codec, clamping an out-of-range level, zstd `0` as uncompressed, and the C `zstd` library.
- [ ] `crates/nvs-stdlib/src/zip.rs:1` and `crates/nvs-stdlib/src/compress.rs:1`: each module doc
  points to its new rule. Each is rewritten as a whole (conventions.md § *A comment*).
- [ ] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. Core\Compress::compress and ::compressor take a level

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/compress.rs:253` and `:279`: `?uint $level = null` last on `compress`
  and `compressor`, with its `ParamDoc` on both cards. The compressor keeps it in a new slot.
- [ ] `crates/nvs-stdlib/src/compress.rs:889` `encode`: takes the level, checks it against the
  codec's range, and throws `RuntimeError` naming the codec and the range. `null` gives today's
  encoder settings exactly.
- [ ] Rust tests: each codec at its lowest, default and highest level round-trips through
  `decompress`, `null` writes the same bytes as before this stage, and each out-of-range level throws.
  The encoder peaks at brotli 5 and 11 and at deflate 1 and 9 are measured and written into the rule.
- [ ] `tests/conformance/core/compress-a-level-is-the-codec-s-own-number.nvst:1`: the check
  `compress-level` runs it.

### 3. one crate frames a zip, and the spreadsheet writer uses it

**Split:** none, one session.

- [ ] `crates/nvs-zip-frame/Cargo.toml:1` and `crates/nvs-zip-frame/src/lib.rs:1`: the framing moved
  from `extensions/spreadsheet/src/zip.rs`, with a level per entry, a time and a zip64 switch as
  parameters, and the zip64 records and the stored-block path added. A `cfg(test)` threshold stands in
  for 4 GiB and 65535.
- [ ] Its unit tests: each record's fields at their APPNOTE offsets, a stored entry, a deflated entry
  at levels 1 and 9, stored deflate blocks at level 0, and zip64 past the test threshold for the entry
  count, an offset and one entry's size.
- [ ] `extensions/spreadsheet/Cargo.toml:14` depends on `nvs-zip-frame` by path, and
  `extensions/spreadsheet/src/zip.rs` is deleted. Its callers use the crate with the fixed time and
  zip64 off.
- [ ] The spreadsheet's own tests, by name: `cargo test --manifest-path extensions/spreadsheet/Cargo.toml`,
  `tests/excel.rs` included, are green, and the bytes of a written workbook are unchanged.

### 4. Core\Zip::createForFile, Writer::add and Writer::finish write an archive to a file

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/zip.rs`: `refuse_hostile` is callable from the writer, and the writer's
  three checks (a repeated name, `\`, a trailing `/`) sit beside it.
- [ ] `crates/nvs-stdlib/src/zip.rs`: the `Core\Zip\Writer` class, its registry rows and reference
  cards (`rule:core-api/reference-card`), `createForFile` with the `.part` file and rename, `add` over
  every `Core\Bytes::Source` and `Core\Bytes::Chunks`, the level checked against deflate's range from
  stage 2, and `finish`. If the writer makes `zip.rs` pass about 2500 lines, it goes into
  `crates/nvs-stdlib/src/zip/write.rs`.
- [ ] `crates/nvs-stdlib/src/registry.rs:2548`: the door row of `createForFile` names `FsWrite`.
- [ ] Rust tests in the writer: every source kind round-trips through `directory` and `read_from`, a
  failed write leaves no `.part` file, and the compressor's size is measured and written into the
  doc comment.
- [ ] `tests/conformance/core/zip-writer-an-archive-written-to-a-file-reads-back.nvst:1` and
  `tests/conformance/core/zip-writer-a-name-the-reader-refuses-throws-when-it-is-added.nvst:1`: the
  checks `writer-round-trip` and `writer-names` run them.

### 5. Core\Zip::createForDownload streams an archive to the browser

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/response.rs`: the RFC 6266 `Content-Disposition` builder, and its test
  against the names `Workbook.nvs`'s `disposition` is tested with.
- [ ] `crates/nvs-stdlib/src/zip.rs`: `createForDownload` opens the response stream with `$mustEnd`
  set to `$name`, and `finish()` ends it.
- [ ] `tests/conformance/core/zip-writer-a-download-streams-an-archive.nvst:1`: a routed case through
  `Core\Test::request` whose body reads back through `Core\Zip::entries`.
- [ ] `tests/conformance/core/zip-writer-a-download-never-finished-is-cut-off.nvst:1`: the check
  `download` runs it.

### 6. extract holds one block, and Core\Zip::stream gives one entry as chunks

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/zip.rs:1528`: `extract` writes each entry through `Budget::stream_from`,
  and the `read_from` call there is gone. The `expect` reason at `zip.rs:1030` is rewritten.
- [ ] `crates/nvs-stdlib/src/zip.rs`: `Core\Zip::stream` as an iterable `Core` class that keeps the
  archive source and the decoder between chunks, modelled on `Core\Http\Chunks` (`http.rs`). Its
  row, its card, and its read of `Core\Request\Part` and `Core\Request\BodyStream` follow
  `rule:core-api/a-member-takes-the-source-the-program-holds`.
- [ ] Rust tests: a bomb entry stops at the bound inside `stream`, a CRC mismatch throws from the last
  read, and `extract`'s peak memory for one entry of 32 MiB stays within two blocks.
- [ ] `tests/conformance/core/zip-stream-gives-one-entry-as-chunks.nvst:1`: an entry piped into
  `Core\IO::writeStream` and into `Core\Hash::of`.

### 7. the feature proofs of every new or changed member

**Split:** one writing subagent per member, side by side, none of them builds. Each gets the member's
signature and card from stages 2 to 6, § *Text an end user reads* in AGENTS.md, and one existing
member's proofs as the model (`bun nv proofs --id 'Core\Zip::read'`). The session builds and runs
`bun nv proofs --comments` over every new or changed `.nvs` once they all end.

- [ ] `Core\Zip::createForFile`: `docs/examples/core/Zip/createForFile/` (about.md, three examples:
  zip a folder of files, one entry at level 0, a name that throws),
  `benches/members/core/Zip/createForFile.nvs` and its `.scale.nvs`, and
  `tests/hostile/core/Zip/createForFile/01-names-from-a-request.nvs`.
- [ ] `Core\Zip::createForDownload`: `docs/examples/core/Zip/createForDownload/` (about.md, three
  examples: a download of a user's files, a download built from a query, what the browser gets when
  the program stops early), its bench, and
  `tests/hostile/core/Zip/createForDownload/01-file-names-that-break-headers.nvs`, modelled on the
  workbook's attack of the same name.
- [ ] `Core\Zip::stream`: `docs/examples/core/Zip/stream/` (about.md, three examples: save one big
  entry to a file, upload it to storage, hash it), its bench, and
  `tests/hostile/core/Zip/stream/01-an-entry-that-grows-past-its-bound.nvs`.
- [ ] `Core\Zip\Writer::add`: `docs/examples/core/Zip-Writer/add/` (about.md, three examples: bytes,
  an open file, a generator of chunks), its bench with growth by entry size, and
  `tests/hostile/core/Zip-Writer/add/01-a-chunk-stream-past-four-gibibytes.nvs` on the test threshold
  if the binary has one, or on the entry count otherwise.
- [ ] `Core\Zip\Writer::finish`: `docs/examples/core/Zip-Writer/finish/` (about.md, three examples:
  the size it returns, `add` after `finish`, a second `finish`), its bench, and
  `tests/hostile/core/Zip-Writer/finish/01-a-writer-never-finished.nvs`.
- [ ] `Core\Compress::compress` and `Core\Compress::compressor`: each `about.md` under
  `docs/examples/core/Compress/` says what `$level` does and gives the table's ranges. One existing
  example of each is rewritten to pass a level, so each member keeps three examples. Each bench gains
  a level-9 row beside its default row.
- [ ] The benches that build archives by hand (`benches/members/core/Zip/entries.scale.nvs:10`,
  `extract.scale.nvs`, `read.scale.nvs`) build them with `Core\Zip::createForFile`, and their
  figures are measured again.
- [ ] `bun nv proofs --owed --goal zip-writer` names nothing, and the check `proofs` is green. Each
  feature it names is paid here, new or changed, listed above or not.

### 8. the help, the reference and the website say what Core\Zip writes and what a level does

- [ ] `crates/nvs-stdlib/src/zip.rs:170`: the class card says the class reads and writes zip
  archives.
- [ ] `docs/examples/core/Zip/entries/about.md`, `read/about.md` and `extract/about.md` are reread,
  and each says that `Core\Zip::createForFile` writes an archive where the reader needs one.
- [ ] The three examples that embed base64 (`docs/examples/core/Zip/entries/01-list-the-files-in-an-archive.nvs:6`,
  `extract/01-unpack-an-archive-into-a-folder.nvs`, `read/01-read-one-file-from-an-archive.nvs`)
  build their archive with `Core\Zip::createForFile`.
- [ ] `website/src/data/changes.json`: `new` `Core\Zip::createForFile`, `new`
  `Core\Zip::createForDownload`, `new` `Core\Zip\Writer`, `new` `Core\Zip::stream`, `changed`
  `Core\Zip::extract` (one block in memory), `changed` `Core\Compress::compress` and `changed`
  `Core\Compress::compressor` (a level).
- [ ] Every handwritten website page that mentions `Core\Zip` or `Core\Compress` is reread and stamped
  (`bun nv site --stamp`). An `Explore` subagent lists them first.
- [ ] `bun nv site --check stale` and `changes` are green.

### 9. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  `about.md` and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the session that found the gap. Nothing is built yet. Every design choice is above, under
*Standing decisions*. The ones marked as the user's call are not reopened.

### Next

Stage 1 from its first item. The checks name cases and proofs that do not exist yet, and they fail
until stages 2, 4, 5 and 7 write them.

### Backlog
