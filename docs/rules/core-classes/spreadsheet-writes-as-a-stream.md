A workbook is written as a stream to the target it was created for, and **streaming is the only way to
write xlsx**: a row's values and styles are final once written, and there is no in-memory editable mode.
An ods is written from the same calls, and held until `finish()` (below).

**The four targets**, each a `Workbook` static method that takes the workbook options last:

| Method | Where the bytes go |
|---|---|
| `createForDownload(string $name, {…})` | The response. It sets `Content-Disposition` with `$name` and starts `Core\Response::stream` at once, so the headers leave before the first row is written |
| `createForFile(string $path, {…})` | A file, opened with `Core\IO::open` in `Write` mode and written with `Core\IO\File::write` |
| `createForStream(Core\IO\File $file, {…})` | A file the program already opened |
| `createInMemory({…})` | Memory, and `finish()`'s `Written` carries the `bytes` |

The workbook options are `zone` (an IANA name; an `Instant` written with no `zone` throws naming its
column, `rule:core-classes/spreadsheet-value-types`), `format` (`'xlsx'`, `'ods'` or `'csv'`, defaulting
to the name's extension and then to `'xlsx'`), and `properties` (`title`, `subject`, `author`, `company`,
`keywords`, `created`). `Workbook::fromRows(iterable $rows, {…})` is the one-call form: it takes the
workbook and sheet options in one bag, writes one sheet named `sheet` (default `'Sheet1'`), and
returns an `Export`, whose `download`, `save`, `to` and `bytes` name the target last. An array or
an `Iterable` of rows writes every time. An `Iterator`, a generator included, is read once, so a second
write of its `Export` throws `LogicError` saying so.

**Rows go in batches, and compressed bytes come back.** `$sheet->add($row)` queues a row in the Novis
half, and a full batch crosses in one guest call, which returns the deflated bytes ready to leave; the
Novis half sends them on before it takes the next row. `addAll` takes any `iterable`, a generator or a
query result included, and batches it the same way. The guest writes sheet XML as plain text, deflates
it as it goes and frames each zip entry with a data descriptor, so it never knows an entry's size in
advance and never seeks. **Strings are written inline** (`t="inlineStr"`), so no shared-string table
grows with the sheet.

**Sheets are written one after another.** `$book->sheet('B')` finishes sheet A; writing to a finished
sheet throws naming it. `{position: n}` sets the tab order, which `workbook.xml` records at
`finish()`. A sheet name follows Excel's rules — at most 31 characters, none of `: \ / ? * [ ]`, not
empty, no leading or trailing apostrophe, unique in the workbook — and a bad one throws naming the
rule it broke.

**What stays changeable until the sheet is finished** is everything xlsx writes after the rows or in
a part of its own: merged ranges, hyperlinks, notes, data validation, outline groups, Excel tables,
conditional formats, images, charts, page setup and protection
(`rule:core-classes/spreadsheet-sheet-end-parts`). Each may point at any row already written. **What is
fixed before the first row** is what xlsx writes before the rows: frozen panes, column widths and
column styles, which is why they are sheet options and not calls. Row heights, hidden rows and outline
levels are options of `add`, because they are written with the row.

The sheet options are `columns` (`rule:core-classes/spreadsheet-styles-and-defaults`), `header`,
`freeze`, `autofilter`, `headerStyle`, `position`, `hidden` (`true` or `'very'`), `tabColor`, `zoom`,
`gridlines`, `rightToLeft` and `overflow`. **The grid is Excel's**: row 1,048,577 throws naming the
limit, and `{overflow: 'newSheet'}` continues on a sheet named `<name> (2)` with the header row
repeated.

**`finish()`** writes the sheet-end parts, `styles.xml`, `workbook.xml`, the content types and the zip
directory, sends the last bytes, and returns a `Written`: the row count per sheet, the size in bytes,
and the bytes for `createInMemory`. **A download never finished is cut off and logged.**
`createForDownload` opens its stream with `Core\Response::stream`'s `$mustEnd` set to the download's
name, and `finish()` calls the stream's `end()`. When the program stops, however it stops, a stream
opened with `$mustEnd` and not ended has its body aborted, so the browser reports a failed download
and not a broken file, and one error record carrying the name as its `stream` field is written
(`nvs_runtime::Ctx::end_unfinished_stream`). No user code runs then
(`rule:concurrency/cancellation-runs-no-user-code`). It costs one pointer per request, and the name's
bytes for a request that streams a download. A workbook written to a file or to memory and never
finished is not reported. A CSV workbook writes each batch through `Core\Csv::format`, ignores styles,
widths and every sheet-end part, and throws naming the format when a second sheet is opened.

**An ods is held, not streamed.** ods puts data validation before the first sheet, and a merge, a
note, a link and an image inside the cells they cover, so a stream that had sent a sheet's rows could
not add them. The ods writer (`extensions/spreadsheet/src/ods/`) checks each row as xlsx does and keeps
it in the guest, compact, and `finish()` writes the whole package: `mimetype` first and stored, then
`content.xml`, `styles.xml`, `meta.xml`, `settings.xml` and the manifest. Every call before
`finish()` returns no bytes, so no byte of an ods leaves before it. A download's headers still leave
at create. Every style is an automatic style, one for each distinct combination, and a formula is
OpenFormula (`of:=`) with its cached value. Two writes of equal input are equal bytes, with `writer`'s
fixed dates.

The tradeoffs. **Performance:** one guest call per batch, not per cell, and a download starts before
the first row is written. **Usability:** an export is one line, and a stream from a query is a
`foreach`. **Simplicity:** one way to write, no editable mode, and the forward-only rules are these
paragraphs, written down once.

- **Memory:** per writing request, one batch of rows in the Novis half and its sheet XML in the guest,
  the compressor's window, the style combinations seen so far, and the sheet-end parts, all inside
  the request's cap and freed when the request ends. A million rows cost one batch, never the sheet,
  so a large export runs in the request and not in a queue.
- **Memory of an ods:** per writing request, every cell of the workbook, compact, under the request's
  cap and freed at `finish()`. **Latency of an ods:** the whole file leaves at `finish()`. A program
  that exports more rows than the cap holds writes xlsx or CSV.

**On disk.** The streaming writer (`extensions/spreadsheet/src/writer.rs`), the four targets,
`sheet`, `finish`, `Written`, `Sheet`'s `add` and `addAll` in batches of 1,000 rows, `fromRows` and
`Export` (`extensions/spreadsheet/nvs/Workbook.nvs`, `Sheet.nvs`, `Export.nvs`), with every sheet
option above, the sheet-name rules, the row options `height`, `hidden`, `level` and `style`,
`define`, CSV and ods (`extensions/spreadsheet/src/ods/`, `extensions/spreadsheet/src/book.rs`), and
the cut-off and logged download that was never finished
(`Core\Response::stream`'s `$mustEnd` and `Core\Response\Stream::end`,
`crates/nvs-stdlib/src/response.rs`).

**Declined:**
- **A request-end hook that calls `Stream::abort()`.** It runs user code after the handler returned,
  which cancellation forbids.
- **Dropping the promise.** The browser would save a broken file and nothing would say so.
- **An in-memory editable mode beside the stream.** Two ways to write is two sets of rules, and the
  editable one holds the whole sheet, which is the memory an export of user data must not cost.
- **`rust_xlsxwriter` as the writer.** Its constant-memory mode writes each sheet through a
  `std::fs::File`, which a guest with no preopen cannot open; it is a source of chart XML and of
  Excel-made fixtures only.
- **A shared-string table.** It grows with every distinct string, so a text-heavy export would hold
  all its text; inline strings cost a few bytes per cell on disk and nothing in memory.
- **Several sheets open at once.** Each would hold its own pending XML and compressor, and xlsx puts
  sheets in separate entries of one stream, so interleaving would buffer all but one.
- **Writing xls, and encrypted or password-protected files.** xls is a 1997 format nothing should
  produce, and encryption is not a sandboxed writer's job
  (`rule:core-classes/spreadsheet-evaluation-and-roster`).
- **Streaming ods, with the parts that cannot follow their rows refused.** A part that works in one
  format and throws in the other is two APIs. The held workbook costs memory, which is spent to keep
  one API.
