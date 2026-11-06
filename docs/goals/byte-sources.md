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
    status: done
    context: [crates/nvs-stdlib/src/csv.rs, crates/nvs-stdlib/src/json/each.rs, crates/nvs-runtime/src/sequence.rs, crates/nvs-host/src/stack.rs, docs/rules/core-classes/json-whole-and-stream.md, docs/rules/core-api/a-lifetime-is-an-object.md]
  - id: 4
    title: "Core\\IO reads and writes bytes, and standard input is a byte stream"
    status: done
    context: [crates/nvs-stdlib/src/io.rs, docs/rules/types/string-is-utf8.md, docs/rules/core-api/each-door-takes-a-different-thing.md, docs/rules/tooling/no-raw-handle-onto-standard-output.md, crates/nvs-stdlib/tests/capability.rs]
  - id: 5
    title: "an upload part and a request body are sources, and reading their start consumes nothing"
    status: done
    context: [crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/mime.rs, docs/rules/http-server/a-part-is-consumed-in-one-of-three-ways.md, docs/rules/http-server/the-body-is-read-on-demand-under-two-caps.md, docs/rules/http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it.md]
  - id: 6
    title: "Core\\Csv::rows decodes a charset as it reads, and the spreadsheet reader has one CSV path"
    status: done
    context: [crates/nvs-stdlib/src/csv.rs, extensions/spreadsheet/nvs/Reader.nvs, extensions/spreadsheet/nvs/Template.nvs, extensions/spreadsheet/src/lib.rs, docs/rules/core-classes/spreadsheet-has-no-io.md, docs/rules/core-classes/spreadsheet-reads-as-a-stream.md]
  - id: 7
    title: "Core\\Zip, Core\\Hash, Core\\Compress and Core\\Storage::put read from every source"
    status: done
    context: [crates/nvs-stdlib/src/zip.rs, crates/nvs-stdlib/src/hash.rs, crates/nvs-stdlib/src/compress.rs, crates/nvs-stdlib/src/storage.rs, docs/rules/core-classes/decompression-bound.md]
  - id: 8
    title: "Core\\Xml::reader reads a document of any size one window at a time"
    status: done
    context: [crates/nvs-stdlib/src/xml.rs, docs/rules/core-classes/xml-tree-and-stream.md]
  - id: 9
    title: "Core\\Pdf and Core\\Image read every source, and their text is tainted"
    status: done
    context: [extensions/pdf/nvs/Reader.nvs, extensions/pdf/nvs/Pages.nvs, extensions/image/nvs/Image.nvs, crates/nvs-cli/src/extensions.rs, docs/rules/packaging/values-cross-as-handles.md, docs/rules/core-classes/pdf-roster.md]
  - id: 10
    title: "the new members and the alias carry their feature proofs"
    status: done
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
    run: [cargo, test, --test, spreadsheet_reader]
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
17. **The byte source, as built in stage 3.**
    - The source holds no Novis value. `ByteSource::from_arg(ctx, value, member)` returns the source
      and the value every read is handed (the `File`, the `bytes`, or the cursor `sequence::start`
      began), with a reference of its own. The member keeps that value in a slot, as `decodeEach`
      kept its holder, so the request's own release frees it; each read takes it as `holder`. A
      `Value` inside filed Rust state would not be freed by that release.
    - A file is read through a second descriptor made with `try_clone`, which shares the position
      on Unix and Windows. That is how the `Drop` backstop puts a position back with no context.
      Every read still checks the program's handle is open. **What it spends:** one descriptor per
      source of a file.
    - Seekability is `nvs_runtime::os::can_seek`: a regular file by mode on Unix, and
      `GetFileType == FILE_TYPE_DISK` on Windows, where `Metadata::is_file` is true for a pipe and a
      seek on one succeeds and moves nothing.
    - A walk takes the source out of its filed state for one `advance()`, as it takes the parse,
      because the state lives in the context the read needs.
    - The write-only `LogicError` of decision 4 needs `Core\IO\File` to remember its mode, which is
      stage 4's `io.rs` work; `from_arg` reads the mode there once it exists.
18. **The charset of a forward read, as built in stage 6.**
    - `Core\Encoding::StreamDecoder` (`crates/nvs-stdlib/src/encoding.rs`) decodes one chunk at a
      time without replacement. `encoding_rs` carries a cut UTF-8, UTF-16 or table sequence;
      `Ascii` and `Latin1` need no state. A wrong byte throws at its offset in the whole source.
    - **A UTF-8 or UTF-16 byte order mark sets the charset and is dropped, whatever `charset`
      says**, as the WHATWG Encoding Standard's *decode* does. So a UTF-16 export reads with no
      option, and the spreadsheet needs no charset logic of its own. Plain UTF-8 is not decoded:
      each field is checked as it ends, as before, so the default path costs nothing more.
    - `Core\Csv::rows`'s options are `PARSE_OPTIONS` plus `charset`; `PARSE_OPTIONS` is the first
      four of `ROWS_OPTIONS`, so the dialect is written once.
19. **The spreadsheet's sources, as built in stage 6.**
    - `Template::open` keeps `bytes|Core\IO\File`. Its rule refuses a `tainted` template, since
      the filled file copies the template's formulas and links, and a part is always tainted. The
      two untainted members of `Core\Bytes::Source` are exactly what it takes already.
    - `Reader::open` takes `Reader::Source = tainted bytes|Core\IO\File|Core\Request\Part`. A body
      stream joins once `Core\Request\BodyStream::peek` exists, because the Novis half must read
      the start to tell CSV from a workbook. Then `Reader::Source` becomes `Core\Bytes::Source`.
    - The extension host lends a file, a part or a body stream through
      `nvs_stdlib::extension_source_size` and `extension_source_range`
      (`crates/nvs-stdlib/src/byte_source.rs`). Each read is a `ByteSource` of its own, because a
      lent value is valid only for its call. A part is held whole by the first range, under
      `[limits] request_body`, so later ranges cost no wire read.
    - A CSV part is read forward by `Core\Csv::rows`, which consumes it. So `sheets()` and then
      `rows()` on a CSV upload reads it twice, and the second read throws. Recorded in the backlog.
20. **The forward members, as built in stage 7.**
    - `byte_source::for_each_chunk(ctx, value, member, each)` reads a source to its end in one call
      (Hash, Storage). `byte_source::Reader` is an `io::Read` over a source for a codec. It copies
      each chunk once, and counts what the codec took in an `Rc<Cell<u64>>`, which stays readable
      while the codec owns the reader. A failed read or a passed deadline ends the input with
      `Ok(0)`, because the zstd encoder panics on a read error. `Reader::close` gives the kept
      fault, which wins over the codec's message.
    - `bytes` and `string` keep their direct path in all four members, so the common call costs
      nothing more than before.
    - **The running ratio.** `decompress` of a `bytes` value is bounded by its whole length, as
      before. Every other source is bounded while it runs by
      `min(consumed × ratio + 64 KiB, ceiling)`, and at its end by the whole input. The 64 KiB
      floor (`RUNNING_FLOOR`, `crates/nvs-stdlib/src/compress.rs`) covers a frame whose start
      compresses far better than the rest. The decision's "the codec's header floor, as
      `Decompressor` does" had no such floor to copy: `Decompressor` checks once at `finish`.
    - `Storage::put` removes the object it began when its source fails part way, so `get` never
      reads half an upload. With `overwrite`, the object it replaced is already gone.
21. **The archive, as built in stage 7.**
    - `zip::Archive` borrows a `bytes` value for every range, so that call costs what a slice did,
      and reads any other source through `ByteSource::range`. The end record is found in the last
      `22 + 65535` bytes. The directory is walked through `Archive::view`, a window of at least
      `READ_CHUNK` that is read again when a record leaves it, so a hostile `begins` offset costs
      one window and never the file. **What it spends:** one window and one compressed piece per
      call, each about `READ_CHUNK`.
    - Each entry is read through `zip::Window`, an `io::Read` of at most `READ_CHUNK` per range.
      A range that comes back empty before the entry's end, or an entry whose end is past the
      archive's size, is `short`: a `ParseError` naming the entry. A fault the window kept wins
      over the decoder's message. The end that wraps `u64` keeps its pinned message
      (`zip-refuses-an-entry-whose-zip64-size-runs-past-the-address-space.nvst`).
    - The repeated-name refusal runs after the walk, over the names the entries own, because the
      window does not keep a name it has passed. An archive with a repeated name and a hostile
      entry after it is now refused for the hostile entry. Both refuse the whole archive.
22. **The XML reader's window, as built in stage 8.**
    - A `string` keeps its old path: walked where it lies, a cursor in a slot. Every other source
      (`bytes` among them) is a `xml::Window` filed on the request, its key in the reader's new
      `window` slot and its holder in `document`, as `Core\Csv::rows` keeps its source.
    - **The parser was not made resumable.** The scanner marks a read that looked at the end of
      its text (`Reader::starts`, `Reader::reached`). A step so marked, while the source has more,
      is thrown away: more is read and the step runs again from the same place, with the open
      depth and the root flag put back. Each growth reads at least what the window holds, so a
      long node is scanned a constant number of times per byte. A malformed document gives the
      message a `string` gives, at every chunk size (`xml_reader_over_chunks_gives_the_nodes_a_string_gives`).
    - Bytes are UTF-8, or UTF-16 by a byte order mark through `StreamDecoder`. A declared
      `encoding` is checked for bytes only: UTF-8, UTF-16 (LE, BE) and US-ASCII read; any other
      throws `ParseError` naming `Core\Encoding`. A `string` is text already, so its declaration
      is not read, as before. Bytes that are not text throw a `ParseError` naming the offset,
      and every later read throws it again.
    - The reader's parameter arms are `Neutral`, as `parse`'s is: every text it gives is
      `tainted` whatever it read. **What it spends:** the largest node about twice over, plus one
      chunk, per open walk.
23. **The whole-buffer components, as built in stage 9.** This revises decision 5's "manifests do
    not change" and "`Image` reads at `open`".
    - A built-in's manifest writes `Core\Bytes::Source` where its WIT has `list<u8>`
      (`NovisType::Source`, `crates/nvs-ext/src/types.rs`). The checker lowers it to the `Core`
      alias, and the load check refuses it in a result or in a third party's export. The WIT does
      not change.
    - The host walks each argument by its manifest type (`read_sources`,
      `crates/nvs-cli/src/extensions.rs`) and reads each source whole through
      `extension_source_size` and `extension_source_range`, a shape's and a union's fields
      included, so `Image`'s `Origin` and `Pages::merge`'s parts carry the source.
    - **`Pages::of` and `Image::open` keep the source**, and the host reads it at `merge`,
      `encode`, `variants` or `raw`. Reading at `open` needs a `Core` member that turns a source
      into whole `bytes`, which `rule:core-api/the-first-call-is-the-best-one` keeps out. So a file
      is read at that call from its start, and a part must still be the walk's current part.
    - **Tainted:** a PDF's text, metadata, attachments and invoice XML, and an image's EXIF
      fields, always, as the spreadsheet reader's are. A Novis method has no contagious return,
      so "contagious" became "always". An image hash, a placeholder and a drawn diff are computed
      in a fixed format and are trusted with a written reason (`Core\Taint::assertTrusted`), the
      same as `encode`'s bytes.
    - A `.nvsr` body after `--BODY--` may hold bytes that are not UTF-8
      (`nvs_test::request::split_octets`), so a test uploads an image or a PDF as a part.
    - **What it spends:** the source's size twice while the call runs, the host's copy and the
      component's; a part also stays held until the walk advances.
24. **The benches and the ledger, as built in stage 11.**
    - A feature has one bench file, so decision 8's benches replace the old ones: `Mime/detect`
      reads an uploaded part, `Hash/of` a file, `Zip/entries` an archive file. The text and `bytes`
      paths keep their direct code and are covered by their unit tests.
    - `Zip/entries.scale.nvs` keeps the entry count fixed and grows each file's data, so the ramp
      shows memory flat in the archive's size. `entries.nvs` declares `complexity constant`.
    - `SHARED_IMPL` (`tools/nv/proofs/collect.ts`) lists `byte_source.rs`. A Rust file that names
      `byte_source::` hashes with its tier joined, by text, so a file that names it only in tests
      is staled too: the conservative direction. The extensions' members read through the host's
      `extension_source_range` and are not joined; their impl files never name the module.
25. **What stage 11's proofs found.**
    - **A component's source must fit twice.** The host's whole read was checked piece by piece and
      the guest's copy never was, so a Pdf file that fit `[limits] memory` once but not twice
      tripped the debug overshoot backstop. `read_sources` (`crates/nvs-cli/src/extensions.rs`)
      now calls `affordable(size × 2)` before it reads; the hostile case
      `Pdf-Reader/open/03-...-once-but-not-twice.nvs` pins it.
    - **A `constant` bench's `.scale.nvs` is never ramped** (`tools/nv/cmd/scaling.ts:1112`
      returns early). Ramping whole-run totals is wrong for it: a `--request` run loads the whole
      body, and a set-up that builds the input grows, so every count grows with the input. The
      ramp is to run two batch sizes at each input size and judge their difference, the
      per-operation counts, against the class. Measured by hand at 1 KiB and 64 KiB: `detect` is
      1 allocation and 12 bytes per call, `peek` 2 and 72, at both sizes.
    - **`Hash::of` allocates a 64 KiB read buffer per call even for a 360-byte file.** That is
      latency on a hot path. The buffer is sized `min(READ_CHUNK, the file's size)` where the size
      is known.
    - **The first `peek` or `detect` of a part holds a whole wire run** (`Multipart::hold`,
      `crates/nvs-stdlib/src/multipart.rs:400`), and `--request`'s buffered body is one run. On a
      server a run is bounded by the wire chunk, so the hold stays O(in-flight). Accepted.
    - `Zip::entries` on a file allocates up to about 256 KiB per call, flat above that: its
      windows, as decision 21 says.
    - **The ASAN leg runs no `.nvs` program,** so the two chunk attacks on `Csv::rows` run under
      valgrind through `bun nv proofs --run --valgrind`, which reads every hostile case. Stage 13
      runs it for them in WSL. A CI step for it is not added by this goal.

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

- [x] `byte_source.rs` (decision 4) and the six tests of the `byte-source` check.
  `crates/nvs-stdlib/src/byte_source.rs:1`
- [x] `Core\Json::decodeEach` reads through it; its `File` and chunk arms and their copies are
  deleted. `crates/nvs-stdlib/src/json/each.rs:587`
- [x] `Core\Csv::rows` reads through it, takes the forward source, and its `Rows` follows the
  source's taint (decision 2); its own read loop is deleted. `crates/nvs-stdlib/src/csv.rs:1292`,
  `crates/nvs-stdlib/src/registry.rs:3546`
- [x] The three tests of the `csv-forward` check, and conformance cases for an `Iterable` that throws
  on its second chunk and a parse error inside chunk 2. `crates/nvs-stdlib/src/csv.rs:1056`
- [x] One reject conformance case per carrier (decision 15): a `Core\Csv::rows` cell read from a
  `Part`, from `Core\Request::bodyStream()` and from an `Iterable<tainted bytes>`, assigned to a
  plain `string`, does not compile.
  `tests/conformance/reject/a-result-read-from-an-upload-part-is-tainted.nvst:1`
- [x] The rules that quote the old signatures say what is true now.
  `docs/rules/core-classes/json-whole-and-stream.md:10`, `docs/rules/core-api/a-lifetime-is-an-object.md:6`
  - [x] `a-lifetime-is-an-object` and `json-whole-and-stream` name `Csv::rows`' source.
  - [x] `decodeEach`'s row takes `Core\Bytes::Source|Core\Bytes::Chunks`, and its rule bullet says so.
  - [x] `ByteSource::from_arg` reads a `Core\Request\Part` forward, as `content()` does.

### 4. Core\IO reads and writes bytes, and standard input is a byte stream

- [x] `Core\IO\File::readBytes`, and `File` remembers its mode. `crates/nvs-stdlib/src/io.rs:1376`,
  `crates/nvs-stdlib/src/io.rs:1474`
- [x] `Core\IO::readBytes` and `Core\IO::stdinStream`. `crates/nvs-stdlib/src/io.rs:99`,
  `crates/nvs-stdlib/src/io.rs:396`
- [x] `Core\IO::write` and `append` take `string|bytes`. `crates/nvs-stdlib/src/io.rs:113`
- [x] Each text read passes its hint into `text_of`, one sentence each and no dash.
  `crates/nvs-stdlib/src/io.rs:2072`
- [x] Capability rows: `readBytes` and `stdinStream` beside `read` and `stdin`, `File::readBytes`
  with none. `crates/nvs-stdlib/src/registry.rs:1903`
- [x] The seven tests of the `io-bytes` check, run in WSL too, and a conformance case that reads
  back a PNG it wrote. `tests/conformance/core/io-read-bytes-gives-a-binary-file-as-it-is.nvst:1`
- [x] The pinned message in the readLine case. `tests/conformance/core/io-readline-a-line-that-is-not-text-does-not-move-and-a-cr-on-a-chunk-edge-is-one-terminator.nvst:53`

### 5. an upload part and a request body are sources, and reading their start consumes nothing

- [x] The prefix buffer and the held part (decision 6). `crates/nvs-stdlib/src/request.rs:3695`,
  `crates/nvs-stdlib/src/request.rs:4125`
- [x] `Core\Request\Part::peek` (decision 5), and its capability row. `crates/nvs-stdlib/src/request.rs:1435`
- [x] `Core\Request\BodyStream` as a source. `crates/nvs-stdlib/src/request.rs:1183`
- [x] The `Part` and `BodyStream` arms of `byte_source.rs`. `crates/nvs-stdlib/src/byte_source.rs:1`
- [x] `Core\Mime::detect` takes `Core\Bytes::Source` and reads at most its widest-signature
  constant; its card and internal message are rewritten. `crates/nvs-stdlib/src/mime.rs:212`,
  `crates/nvs-stdlib/src/mime.rs:579`
- [x] The tests of the `part-source` and `mime-sources` checks.
- [x] The two upload rules say what is true now.
  `docs/rules/http-server/a-part-is-consumed-in-one-of-three-ways.md:13`

### 6. Core\Csv::rows decodes a charset as it reads, and the spreadsheet reader has one CSV path

- [x] The `charset` option and its three tests. `crates/nvs-stdlib/src/csv.rs:259`
- [x] The spreadsheet's `csvFile`, `head` and `decoded` (decision 7), and `csv.encoding` renamed
  `charset`. `extensions/spreadsheet/nvs/Reader.nvs:25`, `extensions/spreadsheet/nvs/Reader.nvs:436`
- [x] `Reader::open` and `Template::open` take `Core\Bytes::Source` (decision 19).
  `extensions/spreadsheet/nvs/Reader.nvs:42`, `extensions/spreadsheet/nvs/Template.nvs:43`
  - [x] `Reader::open` takes bytes, a file and a part, and the host lends each through the byte source.
  - [x] `Template::open` keeps `bytes|Core\IO\File`, by its taint rule.
  - [x] `Core\Request\BodyStream::peek`, then `Reader::Source` becomes `Core\Bytes::Source`.
- [x] `Engine::sourceBytes` deleted. `extensions/spreadsheet/src/lib.rs:1338`
- [x] The test of the `spreadsheet-csv` check. `crates/nvs-cli/tests/spreadsheet_reader.rs:198`
- [x] The spreadsheet rules say what is true now. `docs/rules/core-classes/spreadsheet-has-no-io.md:34`

### 7. Core\Zip, Core\Hash, Core\Compress and Core\Storage::put read from every source

**Split:** Zip is one item group; Hash, Compress and Storage another. They share no file.

- [x] Zip: the directory through `range()` under `affordable`. `crates/nvs-stdlib/src/zip.rs:759`
- [x] Zip: each entry inflated through a bounded reader over its range. `crates/nvs-stdlib/src/zip.rs:1040`
- [x] Zip: a short range is a `ParseError`; the four tests of `zip-file`. `crates/nvs-stdlib/src/zip.rs:903`
- [x] `Core\Hash::of` and `hmac` through `for_each_chunk`. `crates/nvs-stdlib/src/hash.rs:336`
- [x] `Core\Compress` through `reader()`, with the running ratio. `crates/nvs-stdlib/src/compress.rs:644`
- [x] `Core\Storage::put` through `for_each_chunk`. `crates/nvs-stdlib/src/storage.rs:188`
- [x] The tests of `hash-sources` and `compress-sources`, and `rule:core-classes/decompression-bound`
  states the running ratio. `docs/rules/core-classes/decompression-bound.md:4`

### 8. Core\Xml::reader reads a document of any size one window at a time

- [x] The reader's walk reads through `next_chunk`, holding one window. If that is more than one
  session, the session splits the stage before it starts. `crates/nvs-stdlib/src/xml.rs:2792`
- [x] The declared-encoding check (decision 5), and the three tests of `xml-stream`.
- [x] `rule:core-classes/xml-tree-and-stream` says the stream holds one window of any source.
  `docs/rules/core-classes/xml-tree-and-stream.md:1`

### 9. Core\Pdf and Core\Image read every source, and their text is tainted

- [x] The host reads a source whole through `range()` for a whole-buffer component (decision 23).
  `crates/nvs-cli/src/extensions.rs:838`
- [x] `Core\Pdf`'s `Reader::open` and `Pages::of` take `Core\Bytes::Source`, and their text is
  tainted. `extensions/pdf/nvs/Reader.nvs:21`, `extensions/pdf/nvs/Pages.nvs:15`
- [x] `Core\Image`'s `open`, `info`, `hash`, `placeholder`, `palette` and `compare` take
  `Core\Bytes::Source`; `open` keeps the source (decision 23). `extensions/image/nvs/Image.nvs:64`,
  `extensions/image/nvs/Image.nvs:332`
- [x] The tests of `pdf-file` and `image-file`, and the pinned `info(bytes $data)`.
  `crates/nvs-cli/tests/meta.rs:293`
- [x] `rule:core-classes/pdf-roster` quotes the new signatures. `docs/rules/core-classes/pdf-roster.md:22`

### 10. the new members and the alias carry their feature proofs

**Split:** one writing subagent per member, side by side. None builds.

- [x] `Core\IO\File::readBytes`. `docs/examples/core/IO-File/readBytes/about.md:1`
- [x] `Core\IO::readBytes`, and `docs/reference/core/IO.md` names it. `docs/examples/core/IO/readBytes/about.md:1`
- [x] `Core\IO::stdinStream`. `docs/examples/core/IO/stdinStream/about.md:1`
- [x] `Core\Request\Part::peek`, with checks no `Core` member makes, and
  `docs/reference/core/Request-Part.md` names it. `docs/examples/core/Request-Part/peek/about.md:1`
- [x] `Core\Request\BodyStream::peek`, with an `.nvsr` request body, and
  `docs/reference/core/Request-BodyStream.md` names it. `docs/examples/core/Request-BodyStream/peek/about.md:1`
- [x] The aliases' proofs, as stage 2's roster item decided: an alias is part of its class's card
  and owes none (`tools/nv/proofs/roster.ts:14`), so `bun nv proofs --id` knows no alias.
- [x] `bun nv proofs --comments` over every new `.nvs`, and each line it names written again.

### 11. every source kind has a bench with its growth, and every attack on a source is a hostile case

**Split:** benches and attacks are two writing subagents; their files are disjoint.

- [x] The four benches of decision 8, and their shares with `bun nv scaling --budget --write`,
  which also stores stage 10's `IO/readBytes.scale.nvs` and `IO/stdinStream.scale.nvs`.
  `benches/members/core/Mime/detect.scale.nvs:1`
- [x] `rule:testing/member-perf-ledger` hashes `byte_source.rs` for its consumers.
  `docs/rules/testing/member-perf-ledger.md:1`
- [x] The attacks of decision 9. `tests/hostile/core/IO-File/readBytes/01-a-file-that-changes-while-it-is-read.nvs:1`
- [ ] The ramp judges a `constant` bench's `.scale.nvs` by per-operation counts (decision 25).
  `tools/nv/cmd/scaling.ts:1112`
- [ ] `Hash::of` on a small file allocates no 64 KiB buffer (decision 25).
  `crates/nvs-stdlib/src/byte_source.rs:1`

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
- [ ] `bun nv proofs --run --valgrind --only 'Core\Csv::rows'` in WSL, for the two chunk attacks
  (decision 25).
- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 10 are done, and stage 11's three planned items landed. The benches of decision 8
(`Mime/detect`, `Request-Part/peek`, `Hash/of`, `Zip/entries`) pass `scaling --growth` on the
debug binary, and their shares are stored (decision 24). `SHARED_IMPL` in
`tools/nv/proofs/collect.ts` joins `byte_source.rs` into its consumers' hash, so every figure of
`io.rs`, `request.rs`, `mime.rs`, `hash.rs`, `zip.rs`, `compress.rs`, `csv.rs`, `xml.rs`,
`storage.rs` and `json/each.rs` is stale until stage 13. Fourteen hostile cases cover decision 9,
and all but the two Unix-only ones passed on Windows. One of them found that a component's source
did not have to fit twice, and `read_sources` now checks it (decision 25). Stage 11 owes the two
items decision 25 added. The sweep's reds were stale (no `env-secret-proofs` check; Miri ran
at `26dda366`). `scaling --budget --write` still exits 1 on benches this goal did not touch (many
`novis/Spreadsheet-*`, `core/Cap/has`, `core/Metrics/*`, the pipeline operator and its twin,
`Image-Image/compare`), measured on a release binary older than the tree.

May go red at the gate: the cards, meta, LSP stub, `nvs agent` and perf-ledger snapshots of every
widened member (`Xml::reader`, `Zip`, `Hash`, `Compress`, `Storage::put`, `BodyStream::peek`,
`Csv::rows`' `charset`, `Mime::detect`, `Part`, spreadsheet `Reader::open`, and stage 9's
`Image`, `Codec`, Pdf `Reader`, `Pages`, `Engine`); `meta.rs` and `ext_world.rs` export lists;
spreadsheet cases quoting the old CSV path; `llms-full.txt` (`sourceBytes`); a pin of
`part_parse`'s old message; the stage 2 to 4 snapshots (`decodeEach`, `Core\IO`,
`render.test.ts`); examples that put a PDF's text or an image's `exif` in a plain `string`;
`crates/nvs-ext/tests/builtin.rs`' manifest size figures.

### Next

**The end of stage 11**, one session with no subagents, then stage 12 in the same session if it
fits.

- [ ] The ramp judges a `constant` bench's sibling by per-operation counts. `tools/nv/cmd/scaling.ts:1112`
- [ ] `Hash::of` on a small file sizes its buffer to the file. `crates/nvs-stdlib/src/byte_source.rs:1`

The ramp: at each input size, run two batch sizes and judge their difference against the class
(decision 25). `Mime/detect`, `Request-Part/peek` and `Zip/entries` are the three constant benches
with a sibling, and each should pass once it lands. Update `benches/members/README.md` where it
says `constant` needs nothing more, and add a test beside the ramp's existing tests under
`tools/nv/test/`. The buffer: `next_chunk` reads a file into a reused buffer; size it to
`min(READ_CHUNK, size)` when the size is known, then recheck `Hash/of.nvs`'s
`// bench: bytes` and `allocations` lines with `scaling --growth`. Then stage 12.

### Backlog

- `Image::open` keeps a file or a part until `encode` (decision 23). Reading at `open` needs a
  way to turn a source into `bytes` inside a component's Novis half that is not a `Core` member.
- `Pdf\Document::asset`, `Spreadsheet\Sheet::image`, `Font::fromBytes`, the image overlay's `font`,
  QR and barcode inputs take `bytes` the program built; they were not judged against the criterion.
- `Core\Http\Stream` as a source (a download's start, held for a random read) needs the HTTP client's
  own bound; it fits `Chunks` today.
- A CSV upload given to `Reader::open` is read forward by `Core\Csv::rows`, which consumes the part.
  `sheets()` and then `rows()` reads it twice, and the second read throws. A part held whole by a
  random read could be read forward again from the held bytes (decision 19).
- `Core\Storage::get` returns whole bytes; a seekable `File` for a stored object would join every
  member at once (decision 3).
- The new rule `core-api/the-first-call-is-the-best-one` holds for every `Core` member. A review of the rest
  of `Core` against it is the user's to fire.
- `nvs agent find` reads `members` and `enums` only (`crates/nvs-cli/src/agent.rs:1450`), so a
  class's `types`, a `Core` alias or a component's shape, cannot be found by it.
- A cursor on `Order::Meta` in type position for a source alias reaches `Target::Constant`, not
  `Target::TypeAlias` (`crates/nvs-lsp/src/definition.rs:944`); the `Core` alias's hover is answered
  in `card.rs`' `core_member_hover`.
