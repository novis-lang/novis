---
looper: 1
title: Every Core member that reads bytes takes them from where they are
summary: "Reached when a binary file is read as bytes, an upload's first bytes are read without consuming it, Core\\Csv, Core\\Json, Core\\Zip, Core\\Pdf and Core\\Image read from the sources their reads need through one byte source, and no example works around a missing binary read."
context: [crates/nvs-stdlib/src/registry.rs, docs/rules/core-api/one-paradigm-per-operation.md, docs/rules/core-classes/json-whole-and-stream.md]
stages:
  - id: 1
    title: "the rule, and one byte source under Core\\Csv and Core\\Json"
    status: open
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/json/each.rs, crates/nvs-runtime/src/sequence.rs, docs/rules/core-api/a-lifetime-is-an-object.md]
  - id: 2
    title: "Core\\IO reads and writes bytes"
    status: open
    context: [crates/nvs-stdlib/src/io.rs, docs/rules/types/string-is-utf8.md, docs/rules/core-api/each-door-takes-a-different-thing.md]
  - id: 3
    title: "an upload part gives its first bytes without being consumed"
    status: open
    context: [crates/nvs-stdlib/src/request.rs, docs/rules/http-server/a-part-is-consumed-in-one-of-three-ways.md, docs/rules/http-server/the-body-is-read-on-demand-under-two-caps.md]
  - id: 4
    title: "Core\\Csv::rows decodes a charset one chunk at a time, and the spreadsheet reader uses the shared source"
    status: open
    context: [crates/nvs-stdlib/src/csv.rs, extensions/spreadsheet/nvs/Reader.nvs, extensions/spreadsheet/src/lib.rs, docs/rules/core-classes/spreadsheet-has-no-io.md, docs/rules/core-classes/spreadsheet-reads-as-a-stream.md, docs/rules/core-api/no-encoding-argument.md]
  - id: 5
    title: "Core\\Zip reads an archive from a file"
    status: open
    context: [crates/nvs-stdlib/src/zip.rs, docs/rules/core-classes/decompression-bound.md]
  - id: 6
    title: "Core\\Pdf and Core\\Image open a file through the lent source"
    status: open
    context: [extensions/pdf/nvs/Reader.nvs, extensions/pdf/nvs/Pages.nvs, extensions/image/nvs/Image.nvs, crates/nvs-cli/src/extensions.rs, docs/rules/packaging/values-cross-as-handles.md]
  - id: 7
    title: "the three new members carry their feature proofs"
    status: open
    context: [benches/members/README.md, tests/hostile/README.md, docs/rules/testing/feature-proofs.md]
  - id: 8
    title: "the examples read real files and real prefixes"
    status: open
    context: [docs/examples/core/Serialize/decode/03-load-saved-jobs-from-a-file.nvs, docs/agent/prose.md]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: one-source
    stage: 1
    name: one byte source reads a file, chunks and bytes, and Csv::rows reads an upload part through it
    run: [cargo, test, --lib, "source::"]
    timeout: 1800
    tests:
      - forward_source_reads_a_file_from_its_position_and_leaves_it_where_reading_stopped
      - forward_source_reads_chunks_and_bytes_alike
      - random_source_puts_the_file_position_back
  - id: csv-forward
    stage: 1
    name: Csv::rows reads chunks and bytes as it reads a file
    run: [cargo, test, --lib, "csv::"]
    timeout: 1800
    tests:
      - csv_rows_reads_an_iterable_of_chunks_one_record_at_a_time
      - csv_rows_reads_a_record_cut_across_two_chunks
  - id: io-bytes
    stage: 2
    name: a binary file is read and written as bytes, whole and in parts
    run: [cargo, test, --lib, "io::"]
    timeout: 1800
    tests:
      - file_read_bytes_gives_octets_that_are_not_utf8_as_they_are
      - file_read_bytes_gives_empty_bytes_at_the_end
      - io_read_bytes_reads_a_whole_binary_file
      - io_write_and_append_take_bytes
  - id: part-peek
    stage: 3
    name: peek gives a part's first bytes, and every consumer still gives the whole part
    run: [cargo, test, --lib, "request::"]
    timeout: 1800
    tests:
      - part_peek_gives_the_first_bytes_and_content_still_gives_all_of_them
      - part_peek_then_save_to_writes_the_whole_part
      - part_peek_after_content_started_throws
  - id: csv-charset
    stage: 4
    name: Csv::rows decodes a charset one chunk at a time, a character cut across chunks included
    run: [cargo, test, --lib, "csv::"]
    timeout: 1800
    tests:
      - csv_rows_decodes_latin1_one_chunk_at_a_time
      - csv_rows_decodes_a_utf16_character_cut_across_two_chunks
  - id: spreadsheet-csv
    stage: 4
    name: the spreadsheet reader reads a Latin-1 CSV file without holding it whole
    run: [cargo, test, --test, spreadsheet_read]
    timeout: 1800
    tests:
      - a_latin1_csv_file_is_read_without_holding_it_whole
  - id: zip-file
    stage: 5
    name: Zip lists, reads and extracts an archive from a file and puts its position back
    run: [cargo, test, --lib, "zip::"]
    timeout: 1800
    tests:
      - zip_entries_reads_a_file_without_holding_it_whole
      - zip_read_from_a_file_puts_its_position_back
      - zip_extract_from_a_file_keeps_both_bounds
  - id: pdf-file
    stage: 6
    name: a PDF opens from a file
    run: [cargo, test, --test, pdf]
    timeout: 1800
    tests:
      - a_pdf_opens_from_a_file
  - id: image-file
    stage: 6
    name: an image opens and is described from a file
    run: [cargo, test, --test, image_decode]
    timeout: 1800
    tests:
      - an_image_opens_from_a_file
      - image_info_reads_a_file
---

# Every Core member that reads bytes takes them from where they are

## Why

The user asked on 2026-11-06 whether members that take `bytes`, such as `Core\Mime::detect`, can
take a stream. A read of the code found a larger hole: **Novis cannot read a binary file.**
`Core\IO::read`, `readText` and `Core\IO\File::read` give a `string`, and `File::read` throws on
octets that are not UTF-8 (`crates/nvs-stdlib/src/io.rs:2542`). `File::write` takes `string|bytes`,
so a program can write a PNG and never read it back. `Core\Storage::get` is the only way from disk to
`bytes`, and it needs a configured disk and a key, not a path.

What works around the hole today:

- `extensions/spreadsheet/nvs/Reader.nvs:436` (`csvFile`) tells a workbook from a CSV file by calling
  `read(4)` and catching the `RuntimeError`, seeking back by hand.
- The spreadsheet has its own host import, `Engine::sourceBytes` (`Reader.nvs:470`, `:543`), and reads
  a CSV that is not UTF-8 whole into memory, "because `Core\IO\File` reads text only"
  (`docs/rules/core-classes/spreadsheet-has-no-io.md:34`).
- Three private copies of "read raw bytes from an open `File`": `crates/nvs-stdlib/src/csv.rs:1292`,
  `crates/nvs-stdlib/src/json/each.rs:597` and `crates/nvs-stdlib/src/io.rs:2483`. Each has its own
  open check, path retain and error mapping.
- `docs/examples/core/Serialize/decode/03-load-saved-jobs-from-a-file.nvs:24` saves bytes as Base64
  text because `Core\IO::write` takes only a `string`.
- `docs/examples/core/Request-Part/readAll/03-accepting-only-pdf-invoices.nvs:7` reads up to 5 MB of
  an upload to test its first 5 bytes, because nothing gives a part's first bytes without consuming
  it. A forward stream cannot be given to `Mime::detect` either: the detection would consume the
  first chunk, and a later `saveTo` would write the file without its header.
- No Mime, Zip, Image or Hash example reads a real file. About twenty build their input from hex or
  Base64 literals, and three say that a real program reads it from a file.

The members also take their input in four shapes, each chosen by its own class: `bytes` only (`Mime`,
`Zip`, `Image`, `Pdf`), `File|Iterable<bytes>` (`Json::decodeEach`), `File` only (`Csv::rows`), and
`bytes|File` (the spreadsheet reader). This goal gives them one rule and one implementation.

## Standing decisions

1. **The rule: a member that reads bytes takes the weakest source its read needs.** Stage 1 writes it
   as `rule:core-api/a-member-takes-the-weakest-source`, with the Declined list of decision 6.

   | The read needs | The parameter | Members |
   |---|---|---|
   | a fixed prefix | `bytes`; the caller passes the prefix | `Core\Mime::detect` |
   | one pass, forward | the forward source | `Core\Json::decodeEach`, `Core\Csv::rows` |
   | random access | the random source | `Core\Zip`, `Core\Pdf`, `Core\Image`, the spreadsheet reader |
   | the whole value, checked before anything is given | `bytes` | `Core\Crypto::open`, `Core\Serialize::decode`, `Core\Uuid::fromBytes` |

2. **The two sources, and one Rust module for both,** `crates/nvs-stdlib/src/source.rs`.
   - **Forward:** `bytes | Core\IO\File | Iterable<bytes>`, with the chunk arms the registry already
     writes for `decodeEach` (`Iterated(TaintedBytes)`, so a plain chunk widens to tainted). A `File` is
     read from its position, neither opened nor closed, and left where the reading stopped, as
     `Core\Csv::rows` does.
   - **Random:** `bytes | Core\IO\File`. A `File` is read by ranges, and its position is put back
     before the call returns, as the spreadsheet reader does.
   - The module owns the open check, the path retain, the `io_failure` mapping, the per-chunk
     `nvs_runtime::affordable` check and the taint. A read from a tainted source is tainted, and a
     `File`'s bytes are plain, as `Core\IO::read`'s text is.
   - The registry names each union once, as a constant in `crates/nvs-stdlib/src/registry.rs`, so
     every signature, card and checker message is the same. Safe default, if a constant cannot be
     shared: each member writes the union from one `const` slice in `source.rs`.
   - The lent source for extensions (`crates/nvs-cli/src/extensions.rs:919`) serves its ranges from
     the random source, so `io.rs:2483`'s copy is deleted.
3. **The new members, exactly:**
   - `Core\IO\File::readBytes(uint $max): bytes` gives up to `$max` bytes from the position, and empty
     `bytes` at the end. It reads in `READ_CHUNK` pieces under `affordable`, as `read` does, and knows
     nothing of characters.
   - `Core\IO::readBytes(string $path): bytes` reads a whole file under `fs.read`. It sits beside
     `Core\IO::read` under `rule:core-api/each-door-takes-a-different-thing`: one gives text, the
     other octets. `Core\Encoding::decodeText` over it is `readText`, and both stay.
   - `Core\IO::write` and `Core\IO::append` take `string|bytes`, as `Core\IO\File::write` does.
   - `Core\Request\Part::peek(uint $max): tainted bytes` gives the part's first `min($max, size)`
     bytes and consumes nothing. `content()`, `readAll` and `saveTo` then give the whole part, the
     peeked bytes first. A second `peek` with a larger `$max` reads further. A `peek` after a
     consuming read has started throws `LogicError`. The peeked bytes count against the request's
     memory, as `readAll`'s do, and are freed once a consumer has passed them. No setting is added.
   - `Core\Csv::rows` takes the forward source and a `charset?: Core\Charset` option, decoded one
     chunk at a time, with a character cut across two chunks carried to the next. UTF-8 stays the
     default and is checked as today. `rule:core-api/no-encoding-argument` covers members that take a
     `string`, and `rows` takes bytes, as `Core\IO::readText` does.
   - `Core\Zip::entries`, `read` and `extract` take the random source. A `File` holds the central
     directory and one entry at a time, never the archive.
   - `Core\Pdf` (`Reader::open`, `Pages::of`) and `Core\Image` (`open`, `info`) take the random source
     through the lent source. A parser that needs one whole buffer reads the file whole through the
     lent ranges, so its memory is what `bytes` costs today; the about page says so. `Image::info`
     reads only the ranges its decoder asks for, where the decoder reads through `Read + Seek`.
4. **The spreadsheet reader moves onto the shared pieces.** `csvFile` reads its 4 bytes with
   `readBytes` and catches nothing. `head` reads with `readBytes` and puts the position back.
   `decoded` hands the file to `Core\Csv::rows` with `charset`, so a Latin-1 CSV is no longer read
   whole. `Engine::sourceBytes` (`extensions/spreadsheet/src/lib.rs:1338`) is deleted once no caller
   is left, with the two rule sentences that describe it.
5. **What the examples use.** An example that reads a file writes that file first, into
   `Core\IO::temporaryDir()`, with `Core\IO::write`, as Serialize/decode/03 already does. If
   `docs/examples/README.md` allows a fixture file beside the example, the session uses that instead.
   The hex and Base64 fixtures in `tests/conformance/`, `tests/hostile/` and `benches/` stay: a case
   is self-contained, and many of them decode text fields, not files. Every `.nvs` and `about.md`
   follows AGENTS.md § *Text an end user reads*.
6. **Declined, and written into the rule's Declined list:**
   - **A stream given to `Core\Mime::detect`.** Detection would consume the first chunk. The prefix
     comes from `readBytes` or `peek`.
   - **An `Iterable<bytes>` arm on `Core\Hash::of` or `Core\Compress::decompress`.** `Hash::stream`
     and `Compress::decompressor` already read in pieces, and a second way in breaks
     `rule:core-api/one-paradigm-per-operation`.
   - **A streaming `Core\Crypto::seal`/`open`.** A chunked AEAD is its own format and its own design.
   - **`peek` on `Core\Request::bodyStream()` and `Core\Http\Stream::chunks()`.** No example or report
     needs one. `Core\Request::bytes()` is the buffered reader, bounded by `[limits] request_body`.
   - **`Core\IO\File::chunks()`.** Every forward-source member takes a `File` directly.
7. **Lanes.** Stage 1 is the keystone and runs first. Stages 2 and 3 touch disjoint files (`io.rs`,
   `request.rs`) and may run as side goals once stage 1 is committed. Stage 4 needs stages 1 and 2.
   Stage 5 needs stage 1. Stage 6 needs stage 1 and touches only the extensions. Stage 7 needs stages
   2 and 3. Stage 8 needs stages 2 to 6.
8. **What this spends.** Nothing per request that is not spent today: `peek` holds at most `$max`
   bytes per part until a consumer passes them, and every other change reads less. Each member's doc
   comment says what it holds, per call.

## Stages

### 1. the rule, and one byte source under Core\Csv and Core\Json

**Does:** lands the keystone every later stage reads from, and deletes two of the three copies.

- [ ] The rule `core-api/a-member-takes-the-weakest-source` (decision 1), with its Declined list
  (decision 6), its data file and the rendered chapter.
  `docs/rules/core-api/a-member-takes-the-weakest-source.md:1`,
  `data/rules/core-api/a-member-takes-the-weakest-source.json:1`
- [ ] `source.rs` with the forward and the random source (decision 2), and its three tests.
  `crates/nvs-stdlib/src/source.rs:1`
- [ ] The two union constants in the registry. `crates/nvs-stdlib/src/registry.rs:1`
- [ ] `Core\Json::decodeEach` reads through the forward source; its `File` and chunk arms are deleted.
  `crates/nvs-stdlib/src/json/each.rs:597`, `crates/nvs-stdlib/src/json/each.rs:613`
- [ ] `Core\Csv::rows` reads through the forward source and takes chunks and `bytes`; its own read
  loop is deleted, and its two tests are written. `crates/nvs-stdlib/src/csv.rs:1292`,
  `crates/nvs-stdlib/src/csv.rs:1384`
- [ ] `rule:core-classes/json-whole-and-stream` names the forward source in place of its own union.
  `docs/rules/core-classes/json-whole-and-stream.md:15`

### 2. Core\IO reads and writes bytes

- [ ] `Core\IO\File::readBytes`, sharing `read`'s chunked loop. `crates/nvs-stdlib/src/io.rs:1376`,
  `crates/nvs-stdlib/src/io.rs:2557`
- [ ] `Core\IO::readBytes`. `crates/nvs-stdlib/src/io.rs:99`
- [ ] `Core\IO::write` and `append` take `string|bytes`. `crates/nvs-stdlib/src/io.rs:113`,
  `crates/nvs-stdlib/src/io.rs:3061`
- [ ] The four tests of the `io-bytes` check, and one conformance case that reads back a PNG it wrote.
  `tests/conformance/core/io-read-bytes-gives-a-binary-file-as-it-is.nvst:1`
- [ ] The `IO` reference card says it reads text and bytes. `docs/reference/core/IO.md:6`

### 3. an upload part gives its first bytes without being consumed

- [ ] `Core\Request\Part::peek` (decision 3), on the part's existing read buffer.
  `crates/nvs-stdlib/src/request.rs:1426`, `crates/nvs-stdlib/src/request.rs:4125`
- [ ] The three tests of the `part-peek` check. `crates/nvs-stdlib/src/request.rs:4125`
- [ ] The rule says `peek` is not one of the three consuming reads.
  `docs/rules/http-server/a-part-is-consumed-in-one-of-three-ways.md:1`

### 4. Core\Csv::rows decodes a charset one chunk at a time, and the spreadsheet reader uses the shared source

- [ ] The `charset` option on `Core\Csv::rows`, and its two tests. `crates/nvs-stdlib/src/csv.rs:259`
- [ ] The spreadsheet reader's `csvFile`, `head` and `decoded` (decision 4).
  `extensions/spreadsheet/nvs/Reader.nvs:436`, `extensions/spreadsheet/nvs/Reader.nvs:465`,
  `extensions/spreadsheet/nvs/Reader.nvs:537`
- [ ] `Engine::sourceBytes` deleted, if no caller is left. `extensions/spreadsheet/src/lib.rs:1338`
- [ ] The test of the `spreadsheet-csv` check. `crates/nvs-ext/tests/spreadsheet_read.rs:1`
- [ ] The two spreadsheet rule fragments say what is true now.
  `docs/rules/core-classes/spreadsheet-has-no-io.md:34`,
  `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:1`

### 5. Core\Zip reads an archive from a file

- [ ] `entries`, `read` and `extract` take the random source; a `File` holds the central directory and
  one entry. `crates/nvs-stdlib/src/zip.rs:158`, `crates/nvs-stdlib/src/zip.rs:1093`
- [ ] The three tests of the `zip-file` check. `crates/nvs-stdlib/src/zip.rs:1093`
- [ ] `rule:core-classes/decompression-bound` says a file source keeps both bounds.
  `docs/rules/core-classes/decompression-bound.md:39`

### 6. Core\Pdf and Core\Image open a file through the lent source

- [ ] The lent source serves its ranges from the random source; `io.rs`'s copy is deleted.
  `crates/nvs-cli/src/extensions.rs:919`, `crates/nvs-stdlib/src/io.rs:2483`
- [ ] `Core\Pdf`'s `Reader::open` and `Pages::of` take `bytes|Core\IO\File`.
  `extensions/pdf/nvs/Reader.nvs:21`, `extensions/pdf/nvs/Pages.nvs:15`
- [ ] `Core\Image`'s `open` and `info` take `bytes|Core\IO\File` (decision 3).
  `extensions/image/nvs/Image.nvs:64`, `extensions/image/nvs/Image.nvs:91`
- [ ] The tests of the `pdf-file` and `image-file` checks. `crates/nvs-ext/tests/pdf.rs:1`,
  `crates/nvs-ext/tests/image_decode.rs:1`

### 7. the three new members carry their feature proofs

**Split:** one writing subagent per member, side by side, since their files are disjoint. None
builds; the session builds once all three end.

- [ ] `Core\IO\File::readBytes`: what `bun nv proofs --id 'Core\IO\File::readBytes'` names.
  `docs/examples/core/IO-File/readBytes/about.md:1`
- [ ] `Core\IO::readBytes`: what `bun nv proofs --id 'Core\IO::readBytes'` names.
  `docs/examples/core/IO/readBytes/about.md:1`
- [ ] `Core\Request\Part::peek`: what `bun nv proofs --id 'Core\Request\Part::peek'` names. Its
  examples include the PDF check that leaves `readAll/03`.
  `docs/examples/core/Request-Part/peek/about.md:1`
- [ ] `bun nv proofs --comments` over every new `.nvs`, and each line it names written again.

### 8. the examples read real files and real prefixes

**Split:** writing subagents by family, side by side: Mime; Zip; Csv, Image and Pdf about pages;
the rest. None builds.

- [ ] Serialize/decode/03 writes and reads bytes, with no Base64.
  `docs/examples/core/Serialize/decode/03-load-saved-jobs-from-a-file.nvs:24`
- [ ] Request-Part/readAll/03 shows a use that needs the whole part.
  `docs/examples/core/Request-Part/readAll/03-accepting-only-pdf-invoices.nvs:5`
- [ ] Request/files/03 uses `peek` if it needs only a prefix.
  `docs/examples/core/Request/files/03-checking-an-avatar-before-it-is-stored.nvs:18`
- [ ] Compress-Compressor/add/03 reads a real file block by block with `readBytes`.
  `docs/examples/core/Compress-Compressor/add/03-compress-a-large-file-block-by-block.nvs:5`
- [ ] Hash-Digest/03 hashes a file with `Core\Hash::stream` and `readBytes`.
  `docs/examples/types/Hash-Digest/03-checking-a-download-against-its-checksum.nvs:9`
- [ ] Charset/01 reads its Latin-1 bytes from a file.
  `docs/examples/types/Charset/01-reading-text-that-is-not-utf-8.nvs:5`
- [ ] Mime: detect/03 uses `peek`; Mime-Type/01 and mediaType/03 read a file's first bytes; the
  detect about page says how to check a file on disk. `docs/examples/core/Mime/detect/about.md:1`,
  `docs/examples/core/Mime/detect/03-accept-only-images.nvs:22`,
  `docs/examples/types/Mime-Type/01-what-are-these-bytes.nvs:2`,
  `docs/examples/core/Mime/mediaType/03-content-type-for-a-download.nvs:12`
- [ ] Zip: one example each of `entries`, `read` and `extract` opens a file, and each about page
  says a file is accepted. `docs/examples/core/Zip/entries/about.md:1`
- [ ] The about pages of `Core\Csv::rows`, `Core\Image` `open`/`info` and `Core\Pdf` `Reader::open`
  name the sources they take, and Csv/rows gains one example over an upload part.
  `docs/examples/core/Csv/rows/about.md:1`
- [ ] `bun nv proofs --comments` over every changed `.nvs`, and each line it names written again.

### 9. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written by the interactive session of 2026-11-06, from a read of the code, the corpus and the docs.
Nothing of it is built. The chain is `setup-in-any-order` (live) then this goal.

### Next

**Stage 1**, over `source.rs`, `csv.rs`, `json/each.rs` and the registry; no gate.

- [ ] The rule. `docs/rules/core-api/a-member-takes-the-weakest-source.md:1`
- [ ] `source.rs` and its tests. `crates/nvs-stdlib/src/source.rs:1`
- [ ] The union constants. `crates/nvs-stdlib/src/registry.rs:1`
- [ ] `decodeEach` onto the forward source. `crates/nvs-stdlib/src/json/each.rs:597`
- [ ] `Csv::rows` onto the forward source. `crates/nvs-stdlib/src/csv.rs:1292`

### Backlog

- The qr, barcode and font inputs (`Font::fromBytes`, the overlay's `font: bytes`) were not read for
  their sources. A session that touches `extensions/image/` lists them first.
