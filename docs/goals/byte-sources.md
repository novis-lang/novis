---
looper: 1
title: Every Core member that reads bytes takes them from where they are
summary: "Reached when a binary file and binary standard input are read as bytes, every Core member that reads bytes takes a Core\\Bytes::Source and decides itself how much to read, taint follows every source, reading the start of an upload consumes nothing, one byte source module serves every member, and no example works around a missing binary read."
context: [crates/nvs-stdlib/src/registry.rs, docs/rules/core-api/one-paradigm-per-operation.md, docs/rules/http-server/an-upload-is-received-only-through-files.md, docs/rules/security/tainted-qualifier.md]
stages:
  - id: 1
    title: "the rules, and taint follows a part, a body stream and tainted chunks"
    status: done
    context: [crates/nvs-types/src/expr/quals.rs, crates/nvs-types/src/expr/args.rs, docs/rules/security/unclassified-parameter-refuses-tainted.md, docs/rules/core-api/no-encoding-argument.md]
  - id: 2
    title: "Core can declare a type alias, and Core\\Bytes::Source and Chunks exist"
    status: done
    context: [crates/nvs-hir/src/aliases.rs, crates/nvs-types/src/enums.rs, crates/nvs-cli/src/meta.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/stubs.rs, tools/nv/renderers/website-core.ts, tools/nv/proofs/roster.ts, docs/rules/types/type-alias.md, docs/rules/types/class-scoped-alias.md]
  - id: 3
    title: "one byte source module, under Core\\Csv and Core\\Json"
    status: open
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/json/each.rs, crates/nvs-runtime/src/sequence.rs, crates/nvs-host/src/stack.rs, docs/rules/core-classes/json-whole-and-stream.md, docs/rules/core-api/a-lifetime-is-an-object.md]
  - id: 4
    title: "Core\\IO reads and writes bytes, and standard input is a byte stream"
    status: open
    context: [crates/nvs-stdlib/src/io.rs, docs/rules/types/string-is-utf8.md, docs/rules/core-api/each-door-takes-a-different-thing.md, docs/rules/tooling/no-raw-handle-onto-standard-output.md, crates/nvs-stdlib/tests/capability.rs]
  - id: 5
    title: "an upload part and a request body are sources, and reading their start consumes nothing"
    status: open
    context: [crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/mime.rs, docs/rules/http-server/a-part-is-consumed-in-one-of-three-ways.md, docs/rules/http-server/the-body-is-read-on-demand-under-two-caps.md, docs/rules/http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it.md]
  - id: 6
    title: "Core\\Csv::rows decodes a charset as it reads, and the spreadsheet reader has one CSV path"
    status: open
    context: [crates/nvs-stdlib/src/csv.rs, extensions/spreadsheet/nvs/Reader.nvs, extensions/spreadsheet/nvs/Template.nvs, extensions/spreadsheet/src/lib.rs, docs/rules/core-classes/spreadsheet-has-no-io.md, docs/rules/core-classes/spreadsheet-reads-as-a-stream.md]
  - id: 7
    title: "Core\\Zip, Core\\Hash, Core\\Compress and Core\\Storage::put read from every source"
    status: open
    context: [crates/nvs-stdlib/src/zip.rs, crates/nvs-stdlib/src/hash.rs, crates/nvs-stdlib/src/compress.rs, crates/nvs-stdlib/src/storage.rs, docs/rules/core-classes/decompression-bound.md]
  - id: 8
    title: "Core\\Xml::reader reads a document of any size one window at a time"
    status: open
    context: [crates/nvs-stdlib/src/xml.rs, docs/rules/core-classes/xml-tree-and-stream.md]
  - id: 9
    title: "Core\\Pdf and Core\\Image read every source, and their text is tainted"
    status: open
    context: [extensions/pdf/nvs/Reader.nvs, extensions/pdf/nvs/Pages.nvs, extensions/image/nvs/Image.nvs, crates/nvs-cli/src/extensions.rs, docs/rules/packaging/values-cross-as-handles.md, docs/rules/core-classes/pdf-roster.md]
  - id: 10
    title: "the new members and the alias carry their feature proofs"
    status: open
    context: [benches/members/README.md, tests/hostile/README.md, docs/rules/testing/feature-proofs.md, data/proofs/policy.json]
  - id: 11
    title: "every source kind has a bench with its growth, and every attack on a source is a hostile case"
    status: open
    context: [benches/members/README.md, tests/hostile/README.md, docs/rules/testing/member-perf-ledger.md]
  - id: 12
    title: "the examples, cards and rules show the direct call"
    status: open
    context: [docs/examples/README.md, docs/agent/prose.md]
  - id: 13
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: taint-carriers
    stage: 1
    name: a part, a body stream and an iterable of tainted chunks carry taint into a contagious result
    run: [cargo, test, --lib, "quals::"]
    timeout: 1800
    tests:
      - a_request_part_argument_carries_taint
      - an_iterable_of_tainted_bytes_carries_taint
      - a_plain_file_argument_carries_no_taint
  - id: core-alias
    stage: 2
    name: a Core alias resolves where a class-scoped alias does, and messages and hovers keep its name
    run: [cargo, test, --test, class_scoped_aliases]
    timeout: 1800
    tests:
      - a_core_alias_resolves_as_a_source_alias_does
      - a_message_names_a_core_alias_by_its_name
  - id: byte-source
    stage: 3
    name: one byte source reads a file, chunks and bytes, lends each chunk, and puts a file's position back on every way out
    run: [cargo, test, --lib, "byte_source::"]
    timeout: 1800
    tests:
      - next_chunk_reads_a_file_from_its_start_in_one_reused_buffer
      - next_chunk_reads_chunks_and_bytes_alike
      - range_finish_puts_the_file_position_back
      - range_puts_the_position_back_after_a_failed_read
      - a_range_read_stops_at_the_memory_limit
      - a_file_that_cannot_seek_is_read_forward_and_buffered_for_its_start
  - id: csv-forward
    stage: 3
    name: Csv::rows reads chunks and bytes as it reads a file, and its cells are tainted from a tainted source
    run: [cargo, test, --lib, "csv::"]
    timeout: 1800
    tests:
      - csv_rows_reads_an_iterable_of_chunks_one_record_at_a_time
      - csv_rows_reads_a_record_cut_across_two_chunks
      - csv_rows_from_tainted_chunks_gives_tainted_cells
  - id: io-bytes
    stage: 4
    name: binary files and standard input are read as bytes, and a write-only file throws one error everywhere
    run: [cargo, test, --lib, "io::"]
    timeout: 1800
    tests:
      - file_read_bytes_gives_octets_that_are_not_utf8_as_they_are
      - file_read_bytes_gives_empty_bytes_at_the_end
      - io_read_bytes_reads_a_whole_binary_file
      - io_write_and_append_take_bytes
      - a_text_read_of_a_binary_file_names_read_bytes
      - stdin_stream_gives_tainted_chunks
      - a_write_only_file_given_to_a_reader_throws_one_logic_error
  - id: part-source
    stage: 5
    name: a part and a body stream give their start without being consumed, and a random read of one is held under request_body
    run: [cargo, test, --lib, "request::"]
    timeout: 1800
    tests:
      - part_peek_gives_the_first_bytes_and_content_still_gives_all_of_them
      - part_peek_then_save_to_writes_the_whole_part
      - part_peek_over_the_memory_ceiling_throws_before_reading
      - a_prefix_read_after_a_consumer_started_throws
      - a_prefix_read_of_a_part_the_walk_passed_throws
      - a_random_read_of_a_part_is_held_under_request_body_and_save_to_still_writes_it
      - a_body_stream_gives_its_start_without_being_consumed
  - id: mime-sources
    stage: 5
    name: detect reads a part, a body stream, a file and bytes, and never reads past its widest signature
    run: [cargo, test, --lib, "mime::"]
    timeout: 1800
    tests:
      - detect_reads_a_part_and_save_to_still_writes_the_whole_part
      - detect_reads_a_file_from_its_start_and_puts_the_position_back
      - no_signature_reaches_past_the_prefix_detect_reads
  - id: csv-charset
    stage: 6
    name: Csv::rows decodes a charset one chunk at a time, a character cut across chunks and a byte order mark included
    run: [cargo, test, --lib, "csv::"]
    timeout: 1800
    tests:
      - csv_rows_decodes_latin1_one_chunk_at_a_time
      - csv_rows_decodes_a_utf16_character_cut_across_two_chunks
      - csv_rows_drops_a_byte_order_mark
  - id: spreadsheet-csv
    stage: 6
    name: the spreadsheet reader reads a Latin-1 CSV file without holding it whole
    run: [cargo, test, --test, spreadsheet_read]
    timeout: 1800
    tests:
      - a_latin1_csv_file_is_read_without_holding_it_whole
  - id: zip-file
    stage: 7
    name: Zip reads an archive from a file by ranges, streams each entry, and a short range is a ParseError
    run: [cargo, test, --lib, "zip::"]
    timeout: 1800
    tests:
      - zip_entries_reads_a_file_without_holding_it_whole
      - zip_read_from_a_file_streams_the_entry_and_puts_the_position_back
      - zip_extract_from_a_file_keeps_both_bounds
      - a_zip_file_truncated_after_its_directory_is_a_parse_error
  - id: hash-sources
    stage: 7
    name: Hash::of reads a file, a part and chunks in pieces
    run: [cargo, test, --lib, "hash::"]
    timeout: 1800
    tests:
      - hash_of_a_file_reads_it_in_pieces
      - hash_of_a_part_equals_hash_of_its_bytes
  - id: compress-sources
    stage: 7
    name: decompress keeps a running ratio over every source, and a bomb in a part or in chunks stops at it
    run: [cargo, test, --lib, "compress::"]
    timeout: 1800
    tests:
      - decompress_reads_a_file_in_pieces_and_keeps_both_bounds
      - a_bomb_in_chunks_stops_at_the_running_ratio
      - compress_of_a_file_equals_compress_of_its_bytes
  - id: xml-stream
    stage: 8
    name: Xml::reader reads a file and chunks one window at a time, and gives what it gives from a string
    run: [cargo, test, --lib, "xml::"]
    timeout: 1800
    tests:
      - xml_reader_over_a_file_holds_one_window
      - xml_reader_over_chunks_gives_the_nodes_a_string_gives
      - xml_reader_refuses_a_declared_encoding_that_is_not_utf8_or_utf16
  - id: pdf-file
    stage: 9
    name: a PDF opens from a file and an upload, and its text is tainted from a tainted source
    run: [cargo, test, --test, pdf]
    timeout: 1800
    tests:
      - a_pdf_opens_from_a_file
      - a_pdf_from_a_part_gives_tainted_text
  - id: image-file
    stage: 9
    name: an image opens and is described from a file and an upload
    run: [cargo, test, --test, image_decode]
    timeout: 1800
    tests:
      - an_image_opens_from_a_file
      - image_info_reads_a_file
      - image_info_of_a_part_then_save_to_writes_the_whole_part
---

# Every Core member that reads bytes takes them from where they are

## Why

The user asked on 2026-11-06 whether members that take `bytes`, such as `Core\Mime::detect`, can
take a stream. A read of the code found a larger hole: **Novis cannot read a binary file.**
`Core\IO::read`, `readText` and `Core\IO\File::read` give a `string`, and `File::read` throws on
octets that are not UTF-8 (`crates/nvs-stdlib/src/io.rs:2542`). `File::write` takes `string|bytes`,
so a program can write a PNG and never read it back. `Core\IO::stdin()` gives a whole `tainted
string`, so a command-line tool cannot read binary input or input larger than memory.

What works around the hole today:

- `extensions/spreadsheet/nvs/Reader.nvs:436` (`csvFile`) tells a workbook from a CSV file by calling
  `read(4)` and catching the `RuntimeError`, seeking back by hand.
- The spreadsheet has its own host import, `Engine::sourceBytes` (`Reader.nvs:470`, `:543`), and reads
  a CSV that is not UTF-8 whole into memory (`docs/rules/core-classes/spreadsheet-has-no-io.md:34`).
- Three private copies of "read raw bytes from an open `File`": `crates/nvs-stdlib/src/csv.rs:1292`,
  `crates/nvs-stdlib/src/json/each.rs:597` and `crates/nvs-stdlib/src/io.rs:2483`. The last has no
  memory check.
- `docs/examples/core/Serialize/decode/03-load-saved-jobs-from-a-file.nvs:24` saves bytes as Base64
  text because `Core\IO::write` takes only a `string`.
- `docs/examples/core/Request-Part/readAll/03-accepting-only-pdf-invoices.nvs:7` reads up to 5 MB of
  an upload to test its first 5 bytes. That works and gives the right answer, so nothing tells the
  program it took the slow way.
- No Mime, Zip, Image or Hash example reads a real file. About twenty build their input from hex or
  Base64 literals.
- The members take their input in four shapes, each chosen by its own class.

The user then set the principle this goal is built on: **the call a program reaches for first must be
the best, safest and fastest one.** So every member takes the value the program already holds and
decides itself how much of it to read. `Part::peek` stays public, for a check no `Core` member makes.

Two independent reviews of the first draft found what this version fixes: taint is checked only
while compiling, so a `Part` argument would have given untainted results; `Core` cannot declare an
alias; `Image::info` reads whole files; a lazy walk needs a resumable read; the extension range read
has no memory check; and the decompression ratio needs the input's length.

## Standing decisions

1. **Three rules, written in stage 1.**
   - The new rule `core-api/the-first-call-is-the-best-one`: the call a program reaches for first is the
     safest and fastest way to do the job. A member takes the value the program holds and decides
     itself how much of it to read. No fast path needs an extra call, a count or a step the caller
     must remember. Where a slower way cannot be removed, it stops at a bound, and its message or
     about page names the direct call. It cites
     `rule:http-server/an-upload-is-received-only-through-files` ("an API whose safe path is selected
     from a fact the caller cannot see is a defect").
   - The new rule `core-api/a-member-takes-the-source-the-program-holds`: the aliases, the kinds and the
     table below, the criterion, and the Declined list of decision 11. **The criterion:** a member is
     widened when it can hold less than its input. A tree parser that holds the whole document anyway
     (`Xml::parse`, `Html::parse`, `Json::decode`, `Csv::parse`) and `Core\Encoding`, whose output is
     as large as its input, keep `string` or `bytes`.
   - `rule:core-api/no-encoding-argument` gains one sentence: a member that reads bytes may take
     `charset?: Core\Charset` to decode them as it reads, as `Core\IO::readText` does.

   | The member reads | It takes | Members |
   |---|---|---|
   | the start | `Core\Bytes::Source` | `Core\Mime::detect`, `Core\Request\Part::peek` (its own part) |
   | once, forward | `Core\Bytes::Source\|Core\Bytes::Chunks` | `Core\Json::decodeEach`, `Core\Csv::rows`, `Core\Hash::of`, `Core\Hash::hmac`, `Core\Compress::compress`, `Core\Compress::decompress`, `Core\Xml::reader`, `Core\Storage::put` |
   | in any order | `Core\Bytes::Source` | `Core\Zip`, `Core\Pdf`, `Core\Image` (`open`, `info`, `hash`, `placeholder`, `palette`, `compare`), the spreadsheet `Reader::open` and `Template::open` |
   | the whole value, checked before anything is given | `bytes` | `Core\Crypto::open`, `Core\Crypto::sign`, `Core\Crypto::verify`, `Core\Serialize::decode`, `Core\Uuid::fromBytes` |

   `string` stays an arm where a member takes one today.
2. **Taint follows every source** (stage 1). Taint is checked only while compiling
   (`rule:security/tainted-qualifier`), and `carries_tainted`
   (`crates/nvs-types/src/expr/quals.rs:131`) sees only tainted scalars, arrays and unions. It learns
   two more carriers: an instance of a `Core` class the registry marks as carrying tainted bytes
   (`Core\Request\Part`, `Core\Request\BodyStream`), and an `Iterable` or `Iterator` of tainted
   elements. A `Core\IO\File` carries none, as `Core\IO::read`'s text has none. Each widened member's
   contagious result is then tainted for a tainted source and plain for a file. `Core\Csv\Rows` is
   parameterised by its cell type, as `Core\Json\Elements<T>` is, so its cells follow the source.
   Safe default if that cannot be built: a `Rows` read from anything but a file has tainted cells.
3. **Two `Core` aliases on `Core\Bytes`** (stage 2).
   - `type Source = tainted bytes|Core\IO\File|Core\Request\Part|Core\Request\BodyStream;` — the
     sources that can give their start without being consumed and can be held for a random read.
   - `type Chunks = array<tainted bytes>|Iterable<tainted bytes>|Iterator<tainted bytes>;` — the
     registry's `Iterated(TaintedBytes)`, which a plain chunk widens to.
   - A forward member writes `Core\Bytes::Source|Core\Bytes::Chunks`, so the one difference a reader
     needs, whether a stream is accepted, stays visible in two words.
   - The registry gains `CoreTy::Alias(name, &CoreTy)`, which wraps each member's own union, so each
     parameter keeps its own qualifier mark (`registry.rs:1327` folds a union's mark from its arms).
     The HIR alias resolver is seeded with `Core` aliases as `nvs-types` seeds `Core` enums
     (`crates/nvs-types/src/enums.rs:197`). Messages, hovers, cards, `nvs meta --json`, the LSP stubs,
     `nvs agent find` and the website show the alias's name, and the alias is still transparent: a
     program's own `Core\Bytes::Source` parameter takes exactly what the union takes.
     `rule:types/type-alias` gains the `Core` case.
   - `Core\Request\BodyStream` is in both aliases' reach, as a class in `Source` and an
     `Iterable<tainted bytes>` in `Chunks`. If the checker refuses that overlap, `Source` keeps it and
     `Chunks` stays as written.
   - A future source joins `Source` only when it can give all three kinds of read. A seekable
     `Core\IO\File` from anywhere (a storage object opened as a file) works with every member at
     once. A forward-only source yields `tainted bytes` chunks and fits `Chunks` with no change.
4. **One Rust module, `crates/nvs-stdlib/src/byte_source.rs`** (stage 3), named so it does not
   collide with `nvs_runtime::source`. `ByteSource::from_arg(ctx, value, member)` reads the argument
   once: which source it is, the open check, the read mode, seekability and the path retain.
   - **The primitive is `next_chunk(&mut self, ctx) -> Option<Chunk>`,** called from the outer frame,
     so a lazy walk (`Csv::rows`, `decodeEach`, `Xml::reader`) stores the source in its table entry
     and reads one chunk per resume. `Chunk` is a guard that lends the bytes and releases a pulled
     Novis value on drop, the one `unsafe` site (`each.rs:621` today). A file is read into one reused
     buffer per source.
   - `for_each_chunk` (Hash, Storage::put) and `reader()` (`io::Read`, for Compress's flate2, brotli
     and ruzstd readers, which run in the native frame) are built on `next_chunk`. `decodeEach` keeps
     its coroutine, which suspends per element for every source, and feeds it chunks from the outer
     frame as today.
   - `prefix(ctx, n)` gives the start and consumes nothing. `range(ctx, offset, len)` and `size()`
     serve random reads, in `READ_CHUNK` pieces under `nvs_runtime::affordable`. `finish()` puts a
     file's position back and returns its error. A `Drop` backstop restores it on a path that never
     reached `finish()`. Unix and Windows both seek and restore, since Windows' positioned read moves
     the cursor.
   - **A file is read whole, from offset 0, by every kind.** Prefix and random reads put its position
     back; a forward read leaves it at the end. So `$f->readBytes(4); Core\Hash::of($f, …)` hashes the
     whole file. A file that cannot seek, such as a FIFO, is read forward from where it is; its start
     is buffered for a prefix read, and a random read holds it whole under `[limits] memory`.
   - A file opened write-only throws one `LogicError` naming its mode, on every platform
     (`Core\IO\File` remembers its mode; `io.rs:1474`).
   - The extensions' lent source (`crates/nvs-cli/src/extensions.rs:921`) serves its ranges from
     `range()`, so `io.rs:2483`'s copy is deleted and gains the memory check.
5. **The members, exactly:**
   - `Core\IO\File::readBytes(uint $max): bytes` gives up to `$max` bytes from the position, and
     empty `bytes` at the end, in `READ_CHUNK` pieces under `affordable`.
   - `Core\IO::readBytes(string $path): bytes` reads a whole file under `fs.read`, beside
     `Core\IO::read` under `rule:core-api/each-door-takes-a-different-thing`. It never preallocates
     from the file's stated size, so `/dev/zero` and a sparse file stop at the memory limit.
   - `Core\IO::stdinStream(): Iterable<tainted bytes>` gives standard input as chunks, under the
     same main-task rule as `Core\IO::stdin()` (`rule:tooling/the-tty-belongs-to-the-main-task`).
     `Core\Hash::of(Core\IO::stdinStream(), …)` is a checksum tool.
   - `Core\IO::write` and `Core\IO::append` take `string|bytes`.
   - The text reads' message for octets that are not UTF-8 names `readBytes` and `readText`; the
     caller passes the hint into `text_of` (`io.rs:2072`), so `stdin` names `stdinStream`.
   - `Core\Request\Part::peek(uint $max): tainted bytes` gives the part's first `min($max, size)`
     bytes and consumes nothing. A second `peek` with a larger `$max` reads further. A `$max` over
     the memory ceiling throws before reading, as `readAll`'s does (`request.rs:4111`). A `peek` after
     a consumer has started, or on a part the walk has passed, throws `LogicError`.
   - `Core\Mime::detect` reads at most `Core\Mime`'s widest signature, a constant a test checks
     against the table.
   - `Core\Csv::rows` takes `charset?: Core\Charset`, decoded one chunk at a time, a character cut
     across two chunks carried over and a byte order mark dropped. The spreadsheet's `csv.encoding`
     option is renamed `charset`, so one option has one name.
   - `Core\Zip` reads the central directory through `range()` and inflates each entry through a
     bounded reader over its range. A short range read is a `ParseError`, never a slice panic.
   - `Core\Compress::decompress` keeps a running ratio over every source: output at most input
     consumed so far times the ratio, plus the codec's header floor, as `Decompressor` does. The
     file system's stated size is never the input length.
   - `Core\Xml::reader` reads one window at a time through `next_chunk`. A document declares UTF-8
     or UTF-16, and any other declared encoding throws `ParseError` naming `Core\Encoding`.
   - `Core\Storage::put` writes a forward source to its disk without holding it.
   - `Core\Pdf` and `Core\Image` parse one whole buffer, so the host reads the source whole at the
     call through `range()` and passes `bytes`, typed as today. Their manifests and WIT do not change.
     `Image` reads at `open`, since it is lazy (`Image.nvs:59`). Their parameters take `tainted`
     bytes and their text results are contagious, as the spreadsheet reader's are.
6. **A part and a body stream as sources** (stage 5).
   - A **prefix read** fills the part's prefix buffer. `content()`, `readAll` and `saveTo` give the
     buffered bytes first. The buffer is freed when the walk advances.
   - A **random read** holds the part whole, under bare `readAll`'s `[limits] request_body` bound.
     The held bytes stay on the part until the walk advances, so `Image::info($part)` and then
     `$part->saveTo(…)` writes the whole file. Past the bound, the error names `saveTo`, a
     `Core\IO\File` and `readAll({ max })`. `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s
     "a part is never held" becomes "held only by a prefix or random read, until the walk advances".
   - A **forward read** consumes the part as `content()` does.
   - `Core\Request\BodyStream` behaves the same over the request body, so a raw `PUT` upload is
     checked with `Core\Mime::detect(Core\Request::bodyStream())`.
7. **One CSV path in the spreadsheet** (stage 6). `csvFile` and `head` read with `readBytes` and catch
   nothing. Every CSV input, whatever its charset, goes through `Core\Csv::rows` with `charset`, so
   the parser is never chosen by the charset. `Engine::sourceBytes`
   (`extensions/spreadsheet/src/lib.rs:1338`) is deleted once no caller is left.
8. **Performance owed** (stage 11). One bench per kind proves its claim with growth: `Mime::detect`
   over a growing upload with `complexity constant`; `Part::peek` constant over a growing part;
   `Hash::of` over a file, linear with constant allocations; `Zip::entries` over a file, memory flat
   in the archive's size. `rule:testing/member-perf-ledger` hashes `byte_source.rs` into every member
   that reads through it, so a change there makes their figures stale. The gate stage records the
   stale figures with `bun nv proofs --record-perf`.
9. **Attacks owed** (stage 11), under `tests/hostile/core/`: a file truncated while it is read; a
   file changed between random reads (Zip, spreadsheet); a seek past the end, then a forward and a
   prefix read; `IO::readBytes('/dev/zero')` and a huge sparse file (Linux leg); `peek` with the
   largest `uint`; `detect($part)` then `saveTo`; `detect` on a part the walk passed; a write-only
   file given to a reader; a Pdf from a file larger than `[limits] memory`; an `Iterable` that throws
   on its second chunk; a CSV parse error inside chunk 2. The last two also run under ASAN and the
   leak guards.
10. **Tests hold on every runner.** A change to paths or file positions runs its tests in WSL with
    `bun nv linux-test` too. `byte_source::` tests use small chunks under `cfg!(miri)`, as
    `csv.rs:1056` does. A test that needs a Novis `Iterable` is a conformance case.
11. **Declined, and written into the rule's Declined list:**
    - **An `Iterable<bytes>` given to a prefix or random member.** A prefix read would consume it, and
      a random read cannot go back. It does not compile.
    - **A temp file for a held part.** An upload has no temp file by rule.
    - **A streaming `Core\Crypto`.** `open` must check the tag before it gives a byte, and Ed25519's
      `sign` and `verify` hash the message twice.
    - **The tree parsers and `Core\Encoding`**, by the criterion in decision 1.
    - **`Core\IO\File::chunks()`.** Every forward member takes a `File` directly.
    - **An interface a Novis class implements to be a source.** A native reader would then run
      compiled code inside a parse, which `rule:core-classes/json-whole-and-stream` avoids. A Novis
      generator fits `Chunks` already.
    - `Core\Hash::stream` and `Core\Compress::compressor`/`decompressor` stay, for data the program
      makes one piece at a time.
12. **Examples.** Every example and about page shows the direct call. A file an example reads is
    written beside it, as `docs/examples/README.md:117` says, or into `Core\IO::temporaryDir()` where
    the tree has no such file. An upload is a `.nvsr` with `--BODY_CRLF--` and a binary part. The hex
    and Base64 fixtures in `tests/conformance/`, `tests/hostile/` and `benches/` stay. Every `.nvs`
    and `about.md` follows AGENTS.md § *Text an end user reads*.
13. **Lanes.** Stages 1, 2 and 3 run in order: taint first, then the alias every signature uses,
    then the module every member reads through. Stage 4 (`io.rs`) and stage 5 (`request.rs`,
    `mime.rs`) touch disjoint files and may run as side goals after stage 3. Stages 6, 7, 8 and 9
    need stages 4 and 5, and touch disjoint files, so they may run side by side. Stage 10 needs 4
    and 5. Stages 11 and 12 need 6 to 9.
14. **What this spends.** A prefix read or a `peek` holds what it read until the walk advances. A
    random read of a part holds the part, which is what `readAll` costs today, and frees it when the
    walk advances. A source holds one reused chunk buffer. Every other change reads less than
    today. Each member's doc comment says what it holds, per call.
15. **The carriers, as built in stage 1.** `carries_tainted` (`crates/nvs-types/src/expr/quals.rs`)
    reads an `Iterable<T>` or `Iterator<T>` whose `T` carries `tainted`, and an instance of a
    `Core` class `nvs_stdlib::registry::carries_tainted_bytes` names. That is `TAINTED_SOURCES`
    (`Part`, `BodyStream`) and every class whose `ITERABLES` row fixes a tainted element:
    `PartContent`, the HTTP client's `Chunks` and `Lines`, and `Json\Pieces`. Without the second
    half, `$part->content()` would launder, since its type is its own class and not
    `Iterable<tainted bytes>`. The wider reach is applied at every caller of `carries_tainted`,
    since each only adds the bit. **The reject conformance cases moved to stage 3:** no `Core`
    member has a contagious parameter that takes a `Part` or a walk of chunks before
    `Core\Csv::rows` takes the forward source, so no program can show the carrier earlier.
16. **The aliases, as built in stage 2.**
    - `nvs_stdlib::registry::ALIASES` is a roster beside `ENUMS`, keyed by the full name
      `Core\Bytes::Source`, and `aliases_of(class)` lists one class's. A `types` field on `CoreClass`
      would have touched every one of its 189 rows, and the owner is already in the name.
    - The checker, not the HIR resolver, reads a `Core` alias: `nvs-types`' `lower_member_type`
      (`crates/nvs-types/src/lower.rs:476`) tries it after a source alias and before an enum. The HIR
      already leaves an `Owner::Name` it does not own to the checker, and that is where `Core` enums
      are seeded too.
    - The name is kept for messages only: `TypeInterner::name_alias` records it, and `describe`
      spells a type, or each part of a union that has all of an alias's members, by the alias's name.
      A union a program writes out by hand is then also shown by the alias's name, since it is the
      same type.
    - A program's own `Core\Bytes::Source` is the alias's `ty` (`tainted bytes|File|Part|BodyStream`).
      A member writes `CoreTy::Alias(name, &its_own_union)` with its own marks, such as
      `Blob(Contagious)` in place of `TaintedBytes`.

## Stages

### 1. the rules, and taint follows a part, a body stream and tainted chunks

**Does:** closes the hole the widened arms would open, before any arm exists.

- [x] The three rules of decision 1, with data files and the rendered chapters.
  `docs/rules/core-api/the-first-call-is-the-best-one.md:1`,
  `docs/rules/core-api/a-member-takes-the-source-the-program-holds.md:1`,
  `docs/rules/core-api/no-encoding-argument.md:1`
- [x] `carries_tainted` learns the two carriers of decision 2, and the registry marks `Part` and
  `BodyStream`. `crates/nvs-types/src/expr/quals.rs:131`, `crates/nvs-types/src/expr/args.rs:747`
- [x] The three tests of the `taint-carriers` check. The reject conformance cases moved to stage 3
  (decision 15). `crates/nvs-types/src/expr/quals.rs:2245`

### 2. Core can declare a type alias, and Core\Bytes::Source and Chunks exist

**Split:** an `Explore` subagent lists every site that renders or indexes a class's `types`, so no
tool is missed.

- [x] `CoreTy::Alias`, its `spelled()` and `classification()`, and the `ALIASES` roster
  (decision 16). `crates/nvs-stdlib/src/registry.rs:1265`, `crates/nvs-stdlib/src/registry.rs:1363`
- [x] The checker reads a `Core` alias where `Owner::Name` is lowered (decision 16).
  `crates/nvs-types/src/lower.rs:476`
- [x] `nvs meta --json` emits a `Core` class's `types`. `crates/nvs-cli/src/meta.rs:251`
- [x] The LSP's card, hover, completion and stubs show the alias.
  `crates/nvs-lsp/src/card.rs:166`, `crates/nvs-lsp/src/stubs.rs:388`
- [x] The website renders the alias with its doc. `tools/nv/renderers/website-core.ts:97`
- [x] The proof roster gives an alias its proofs, or records that it is part of its class's card.
  `tools/nv/proofs/roster.ts:14`
- [x] `Core\Bytes::Source` and `Chunks` (decision 3), and `docs/reference/core/Bytes.md` names them.
  `crates/nvs-stdlib/src/registry.rs:917`
- [x] `rule:types/type-alias` gains the `Core` case. `docs/rules/types/type-alias.md:1`
- [x] The two tests of the `core-alias` check. `crates/nvs-types/tests/class_scoped_aliases.rs:1`

### 3. one byte source module, under Core\Csv and Core\Json

- [ ] `byte_source.rs` (decision 4) and the six tests of the `byte-source` check.
  `crates/nvs-stdlib/src/byte_source.rs:1`
- [ ] `Core\Json::decodeEach` reads through it; its `File` and chunk arms and their copies are
  deleted. `crates/nvs-stdlib/src/json/each.rs:587`
- [ ] `Core\Csv::rows` reads through it, takes the forward source, and its `Rows` follows the
  source's taint (decision 2); its own read loop is deleted. `crates/nvs-stdlib/src/csv.rs:1292`,
  `crates/nvs-stdlib/src/registry.rs:3546`
- [ ] The three tests of the `csv-forward` check, and conformance cases for an `Iterable` that throws
  on its second chunk and a parse error inside chunk 2. `crates/nvs-stdlib/src/csv.rs:1056`
- [ ] One reject conformance case per carrier (decision 15): a `Core\Csv::rows` cell read from a
  `Part`, from `Core\Request::bodyStream()` and from an `Iterable<tainted bytes>`, assigned to a
  plain `string`, does not compile.
  `tests/conformance/reject/a-result-read-from-an-upload-part-is-tainted.nvst:1`
- [ ] The rules that quote the old signatures say what is true now.
  `docs/rules/core-classes/json-whole-and-stream.md:10`, `docs/rules/core-api/a-lifetime-is-an-object.md:6`

### 4. Core\IO reads and writes bytes, and standard input is a byte stream

- [ ] `Core\IO\File::readBytes`, and `File` remembers its mode. `crates/nvs-stdlib/src/io.rs:1376`,
  `crates/nvs-stdlib/src/io.rs:1474`
- [ ] `Core\IO::readBytes` and `Core\IO::stdinStream`. `crates/nvs-stdlib/src/io.rs:99`,
  `crates/nvs-stdlib/src/io.rs:396`
- [ ] `Core\IO::write` and `append` take `string|bytes`. `crates/nvs-stdlib/src/io.rs:113`
- [ ] Each text read passes its hint into `text_of`, one sentence each and no dash.
  `crates/nvs-stdlib/src/io.rs:2072`
- [ ] Capability rows: `readBytes` and `stdinStream` beside `read` and `stdin`, `File::readBytes`
  with none. `crates/nvs-stdlib/src/registry.rs:1903`
- [ ] The seven tests of the `io-bytes` check, run in WSL too, and a conformance case that reads
  back a PNG it wrote. `tests/conformance/core/io-read-bytes-gives-a-binary-file-as-it-is.nvst:1`
- [ ] The pinned message in the readLine case. `tests/conformance/core/io-readline-a-line-that-is-not-text-does-not-move-and-a-cr-on-a-chunk-edge-is-one-terminator.nvst:53`

### 5. an upload part and a request body are sources, and reading their start consumes nothing

- [ ] The prefix buffer and the held part (decision 6). `crates/nvs-stdlib/src/request.rs:3695`,
  `crates/nvs-stdlib/src/request.rs:4125`
- [ ] `Core\Request\Part::peek` (decision 5), and its capability row. `crates/nvs-stdlib/src/request.rs:1435`
- [ ] `Core\Request\BodyStream` as a source. `crates/nvs-stdlib/src/request.rs:1183`
- [ ] The `Part` and `BodyStream` arms of `byte_source.rs`. `crates/nvs-stdlib/src/byte_source.rs:1`
- [ ] `Core\Mime::detect` takes `Core\Bytes::Source` and reads at most its widest-signature
  constant; its card and internal message are rewritten. `crates/nvs-stdlib/src/mime.rs:212`,
  `crates/nvs-stdlib/src/mime.rs:579`
- [ ] The tests of the `part-source` and `mime-sources` checks.
- [ ] The two upload rules say what is true now.
  `docs/rules/http-server/a-part-is-consumed-in-one-of-three-ways.md:13`

### 6. Core\Csv::rows decodes a charset as it reads, and the spreadsheet reader has one CSV path

- [ ] The `charset` option and its three tests. `crates/nvs-stdlib/src/csv.rs:259`
- [ ] The spreadsheet's `csvFile`, `head` and `decoded` (decision 7), and `csv.encoding` renamed
  `charset`. `extensions/spreadsheet/nvs/Reader.nvs:25`, `extensions/spreadsheet/nvs/Reader.nvs:436`
- [ ] `Reader::open` and `Template::open` take `Core\Bytes::Source`.
  `extensions/spreadsheet/nvs/Reader.nvs:42`, `extensions/spreadsheet/nvs/Template.nvs:43`
- [ ] `Engine::sourceBytes` deleted. `extensions/spreadsheet/src/lib.rs:1338`
- [ ] The test of the `spreadsheet-csv` check. `crates/nvs-ext/tests/spreadsheet_read.rs:1`
- [ ] The spreadsheet rules say what is true now. `docs/rules/core-classes/spreadsheet-has-no-io.md:34`

### 7. Core\Zip, Core\Hash, Core\Compress and Core\Storage::put read from every source

**Split:** Zip is one item group; Hash, Compress and Storage another. They share no file.

- [ ] Zip: the directory through `range()` under `affordable`. `crates/nvs-stdlib/src/zip.rs:559`
- [ ] Zip: each entry inflated through a bounded reader over its range. `crates/nvs-stdlib/src/zip.rs:773`
- [ ] Zip: a short range is a `ParseError`; the four tests of `zip-file`. `crates/nvs-stdlib/src/zip.rs:1093`
- [ ] `Core\Hash::of` and `hmac` through `for_each_chunk`. `crates/nvs-stdlib/src/hash.rs:336`
- [ ] `Core\Compress` through `reader()`, with the running ratio. `crates/nvs-stdlib/src/compress.rs:644`
- [ ] `Core\Storage::put` through `for_each_chunk`. `crates/nvs-stdlib/src/storage.rs:188`
- [ ] The tests of `hash-sources` and `compress-sources`, and `rule:core-classes/decompression-bound`
  states the running ratio. `docs/rules/core-classes/decompression-bound.md:4`

### 8. Core\Xml::reader reads a document of any size one window at a time

- [ ] The reader's walk reads through `next_chunk`, holding one window. If that is more than one
  session, the session splits the stage before it starts. `crates/nvs-stdlib/src/xml.rs:2792`
- [ ] The declared-encoding check (decision 5), and the three tests of `xml-stream`.
- [ ] `rule:core-classes/xml-tree-and-stream` says the stream holds one window of any source.
  `docs/rules/core-classes/xml-tree-and-stream.md:1`

### 9. Core\Pdf and Core\Image read every source, and their text is tainted

- [ ] The host reads a source whole through `range()` for a whole-buffer component (decision 5).
  `crates/nvs-cli/src/extensions.rs:838`
- [ ] `Core\Pdf`'s `Reader::open` and `Pages::of` take `Core\Bytes::Source`, and their text is
  contagious. `extensions/pdf/nvs/Reader.nvs:21`, `extensions/pdf/nvs/Pages.nvs:15`
- [ ] `Core\Image`'s `open`, `info`, `hash`, `placeholder`, `palette` and `compare` take
  `Core\Bytes::Source`; `open` reads at the call. `extensions/image/nvs/Image.nvs:64`,
  `extensions/image/nvs/Image.nvs:332`
- [ ] The tests of `pdf-file` and `image-file`, and the pinned `info(bytes $data)`.
  `crates/nvs-cli/tests/meta.rs:293`
- [ ] `rule:core-classes/pdf-roster` quotes the new signatures. `docs/rules/core-classes/pdf-roster.md:22`

### 10. the new members and the alias carry their feature proofs

**Split:** one writing subagent per member, side by side. None builds.

- [ ] `Core\IO\File::readBytes`. `docs/examples/core/IO-File/readBytes/about.md:1`
- [ ] `Core\IO::readBytes`, and `docs/reference/core/IO.md` names it. `docs/examples/core/IO/readBytes/about.md:1`
- [ ] `Core\IO::stdinStream`. `docs/examples/core/IO/stdinStream/about.md:1`
- [ ] `Core\Request\Part::peek`, with checks no `Core` member makes, and
  `docs/reference/core/Request-Part.md` names it. `docs/examples/core/Request-Part/peek/about.md:1`
- [ ] The aliases' proofs, as stage 2's roster item decided. `docs/examples/types/Bytes-Source/about.md:1`
- [ ] `bun nv proofs --comments` over every new `.nvs`, and each line it names written again.

### 11. every source kind has a bench with its growth, and every attack on a source is a hostile case

**Split:** benches and attacks are two writing subagents; their files are disjoint.

- [ ] The four benches of decision 8, and their shares with `bun nv scaling --budget --write`.
  `benches/members/core/Mime/detect.scale.nvs:1`
- [ ] `rule:testing/member-perf-ledger` hashes `byte_source.rs` for its consumers.
  `docs/rules/testing/member-perf-ledger.md:1`
- [ ] The attacks of decision 9. `tests/hostile/core/IO-File/readBytes/01-a-file-that-changes-while-it-is-read.nvs:1`

### 12. the examples, cards and rules show the direct call

**Split:** writing subagents by family, side by side: Mime and Image; Zip; Csv, Hash, Compress and
Storage; IO, Serialize and Charset. None builds.

- [ ] Serialize/decode/03 writes and reads bytes, with no Base64.
  `docs/examples/core/Serialize/decode/03-load-saved-jobs-from-a-file.nvs:24`
- [ ] Request-Part/readAll/03 shows a use that needs the whole part; its PDF check moves to
  `Core\Mime::detect($part)`. `docs/examples/core/Request-Part/readAll/03-accepting-only-pdf-invoices.nvs:5`
- [ ] Request/files/03 checks the avatar with `Image::info($part)` and then saves it.
  `docs/examples/core/Request/files/03-checking-an-avatar-before-it-is-stored.nvs:18`
- [ ] Compress-Compressor/add/03 shows blocks the program makes.
  `docs/examples/core/Compress-Compressor/add/03-compress-a-large-file-block-by-block.nvs:5`
- [ ] Hash-Digest/03 checks a downloaded file with `Core\Hash::of($file)`.
  `docs/examples/types/Hash-Digest/03-checking-a-download-against-its-checksum.nvs:9`
- [ ] Charset/01 reads its Latin-1 bytes from a file. `docs/examples/types/Charset/01-reading-text-that-is-not-utf-8.nvs:5`
- [ ] Mime: detect/03 passes the part; Mime-Type/01 and mediaType/03 pass a file; detect's about
  page names its sources. `docs/examples/core/Mime/detect/about.md:1`
- [ ] Zip: one example each of `entries`, `read` and `extract` opens a file. `docs/examples/core/Zip/entries/about.md:1`
- [ ] The about pages and reference cards of every widened member name their sources: Csv/rows,
  Json/decodeEach, Hash, Compress, Xml/reader, Storage/put, IO/write, IO/append, Image, Pdf.
  `docs/reference/core/Hash.md:6`, `docs/examples/core/IO/write/about.md:1`
- [ ] The doc comments that state the old signatures. `crates/nvs-stdlib/src/zip.rs:1076`
- [ ] `bun nv render --website` regenerates `website/src/data/core.json` and `llms-full.txt`.
- [ ] `bun nv proofs --comments` over every changed `.nvs`, and each line it names written again.

### 13. the goal is verified once, and every failure is fixed

- [ ] `bun nv proofs --record-perf` over the classes whose figures are stale.
- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. `core-alias` passes (`looper check --name core-alias`), and so do
`every_core_alias_has_a_registered_owner_and_classes` (registry) and
`a_core_alias_hover_shows_its_name_its_type_and_its_card` (`nvs-lsp` `card.rs`). Decision 16 records
how the aliases are built: an `ALIASES` roster, read by the checker's `lower_member_type`, with the
name kept in the interner for messages. `nvs meta --json`, the stubs, hover, completion in type
position, the website's class page (its section is now "Types", with each card) and `Bytes.md` show
them. `nvs agent find` does not index a class's `types` yet (backlog).

Tests that may go red at the gate: `tools/nv/test/render.test.ts` if a fixture compares a class's
`types` shape (each now has `descHtml`), and any LSP stub snapshot of `Core\Bytes`'s stub, which now
lists two `type` members.

The sweep's reds were stale. Nightly run `42201104003` ran at 10:14 UTC on `26dda36`, before
`f13f40646a` fixed the Miri timeouts, and no check named `env-secret-proofs` exists in the tree.

### Next

**Stage 3**, `byte_source.rs` under `Core\Csv::rows` and `Core\Json::decodeEach`. Its items are in
the stage, in order.

- [ ] `byte_source.rs` (decision 4) and the six `byte-source` tests. `crates/nvs-stdlib/src/byte_source.rs:1`
- [ ] `decodeEach` and `Csv::rows` read through it. `crates/nvs-stdlib/src/json/each.rs:587`,
  `crates/nvs-stdlib/src/csv.rs:1292`
- [ ] The reject conformance cases of decision 15, over `crates/nvs-types/src/expr/quals.rs:131`.

Two traps for stage 3:

- `CoreTy::Iterated(elem)`'s mark is `None` unless `elem` is `Neutral`
  (`crates/nvs-stdlib/src/registry.rs:1296`). A `Chunks` arm under a contagious `Csv::rows` needs a
  `Contagious` element to answer `Contagious`, as `Array`'s does. Otherwise the arm refuses a tainted walk.
- A member writes `CoreTy::Alias(r"Core\Bytes::Source", &CoreTy::Union(&[Blob(Contagious), …]))`,
  which interns with a plain `bytes` arm. Check that a program's own `Core\Bytes::Source` variable,
  whose arm is `tainted bytes`, is admitted at that parameter. If the union admission does not strip
  the qualifier per arm, fix it in `crate::expr::quals`' admission, not by writing `TaintedBytes`
  into the member's union.

### Backlog

- `Pdf\Document::asset`, `Spreadsheet\Sheet::image`, `Font::fromBytes`, the image overlay's `font`,
  QR and barcode inputs take `bytes` the program built; they were not judged against the criterion.
- `Core\Http\Stream` as a source (a download's start, held for a random read) needs the HTTP client's
  own bound; it fits `Chunks` today.
- `Core\Storage::get` returns whole bytes; a seekable `File` for a stored object would join every
  member at once (decision 3).
- The new rule `core-api/the-first-call-is-the-best-one` holds for every `Core` member. A review of the rest
  of `Core` against it is the user's to fire.
- `nvs agent find` reads `members` and `enums` only (`crates/nvs-cli/src/agent.rs:1450`), so a
  class's `types`, a `Core` alias or a component's shape, cannot be found by it.
- A cursor on `Order::Meta` in type position for a source alias reaches `Target::Constant`, not
  `Target::TypeAlias` (`crates/nvs-lsp/src/definition.rs:944`); the `Core` alias's hover is answered
  in `card.rs`' `core_member_hover`.
