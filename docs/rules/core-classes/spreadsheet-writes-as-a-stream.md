A workbook is written as a stream to the target it was created for, and **streaming is the only way to
write**: a row's values and styles are final once written, and there is no in-memory editable mode.

**The four targets**, each a `Workbook` static method that takes the workbook options last:

| Method | Where the bytes go |
|---|---|
| `createForDownload(string $name, {…})` | The response. It sets `Content-Disposition` with `$name` and starts `Core\Response::stream` at once, so the headers leave before the first row is written |
| `createForFile(string $path, {…})` | A file, through `Core\IO::writeStream` |
| `createForStream(Core\IO\File $file, {…})` | A file the program already opened |
| `createInMemory({…})` | Memory, and `finish()`'s `Written` carries the `bytes` |

The workbook options are `zone` (an IANA name; an `Instant` written with no `zone` throws naming its
column, `rule:core-classes/spreadsheet-value-types`), `format` (`'xlsx'` or `'csv'`, defaulting to the
name's extension and then to `'xlsx'`), and `properties` (`title`, `subject`, `author`, `company`,
`keywords`, `created`). `Workbook::fromRows(iterable $rows, {…})` is the one-call form: it takes the
workbook and sheet options in one bag, writes one sheet named `sheet` (default `'Sheet1'`), and
returns an `Export`, whose `download`, `save`, `to` and `bytes` name the target last.

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
and the bytes for `createInMemory`. **A workbook never finished** is logged as an error naming its
target when the request ends, and a download is cut off, so the browser reports a failed download and
not a broken file. A CSV workbook writes each batch through `Core\Csv::format`, ignores styles, widths
and every sheet-end part, and throws naming the format when a second sheet is opened.

The tradeoffs. **Performance:** one guest call per batch, not per cell, and a download starts before
the first row is written. **Usability:** an export is one line, and a stream from a query is a
`foreach`. **Simplicity:** one way to write, no editable mode, and the forward-only rules are these
paragraphs, written down once.

- **Memory:** per writing request, one batch of rows in the Novis half and its sheet XML in the guest,
  the compressor's window, the style combinations seen so far, and the sheet-end parts, all inside
  the request's cap and freed when the request ends. A million rows cost one batch, never the sheet,
  so a large export runs in the request and not in a queue.

**Not shipped.** Nothing of `Novis\Spreadsheet` is on disk.

**Declined:**
- **An in-memory editable mode beside the stream.** Two ways to write is two sets of rules, and the
  editable one holds the whole sheet, which is the memory an export of user data must not cost.
- **`rust_xlsxwriter` as the writer.** Its constant-memory mode writes each sheet through a
  `std::fs::File`, which a guest with no preopen cannot open; it is a source of chart XML and of
  Excel-made fixtures only.
- **A shared-string table.** It grows with every distinct string, so a text-heavy export would hold
  all its text; inline strings cost a few bytes per cell on disk and nothing in memory.
- **Several sheets open at once.** Each would hold its own pending XML and compressor, and xlsx puts
  sheets in separate entries of one stream, so interleaving would buffer all but one.
- **Writing xls, encrypted or password-protected files, and ods.** xls is a 1997 format nothing should
  produce, encryption is not a sandboxed writer's job, and ods writing is the second wave
  (`rule:core-classes/spreadsheet-evaluation-and-roster`).
