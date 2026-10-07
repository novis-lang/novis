---
milestone: M17
position: last
---
# Loop goal 200 — `Novis\Spreadsheet` is built into every binary: xlsx written and read in constant memory, styled exports from one array, templates filled, and CSV through the same API

The fourth component built into every `nvs` binary. `Novis\Spreadsheet` writes xlsx (and CSV) as a
stream, so a workbook of a million rows costs one batch of rows in memory, never the whole sheet. It
reads xlsx and xlsb as a stream over a file the program opened, and xlsm, xls, ods and CSV beside
them. It fills designed templates, with repeated rows and the formulas and charts under them kept
right. It does no I/O of its own: bytes come back to the Novis half, which writes them where the
program said.

```nvs
use Novis\Spreadsheet\{Workbook, Reader, Template, Style, Formula};   // names are the record's; this is a sketch

Workbook::fromRows($orders)->download('orders.xlsx');     // styled header, frozen, filtered, auto width

$book  = Workbook::createForDownload('orders.xlsx', {zone: 'Europe/Vienna'});   // headers go out now
$sheet = $book->sheet('Orders', {columns: {
    id:    {header: 'Order', width: 10},
    total: {header: 'Total', format: 'money'},
    sum:   {header: 'Sum', formula: fn(int $row) => Formula::of("{$sheet->cell('qty', $row)}*{$sheet->cell('price', $row)}")},
}});
foreach ($db->query('SELECT ...') as $row) { $sheet->add($row); }             // compressed chunks leave per batch
$sheet->add(['total' => Formula::of("SUM({$sheet->column('total')})")], {style: Style::of({bold: true})});
$sheet->merge('A1:B1')->note('C2', 'Checked')->validate('E2:E9999', ...);    // sheet-end parts, any earlier row
$done = $book->finish();                                                      // ->rows, ->size; ->bytes in memory

foreach (Reader::open($file)->rows('Orders', {header: true, text: ['Zip']}) as $line => $row) { ... }
Template::open($tpl)->fill(['number' => 1042, 'items' => $items])->save('/exports/invoice.xlsx');
```

Its scope is § *Standing decisions*, which the user settled question by question when the goal was
written, under the same rule as goal `pdf`: **every common case is in, an edge case is in when it is
easy, and a hard edge case almost nobody needs stays out.** Formula evaluation and writing ods are not
in it.

## Why here

It stands on finished work only: the extension system, the build script and embedding path goals
`ext-image`, `ext-intl` and `pdf` built for three components, guest resources that live for one
request (`crates/nvs-ext/src/call.rs:50-57`, proven by
`tests/conformance/ext/an-extension-resource-is-held-by-the-program-and-passed-back.nvst`),
`Core\Csv::rows` (`crates/nvs-stdlib/src/csv.rs:119-185`), `Core\Response::stream`, `Core\IO::writeStream`
and `Core\IO\File::seek` (`crates/nvs-stdlib/src/io.rs:1404`). Its files are a new
`extensions/spreadsheet`, a new `wit/spreadsheet.wit`, `nvs-ext`'s build and load tables and
`crates/nvs-stdlib/src/zip.rs`'s decompression bound. It shares the build and load tables with goal
`pdf`, so it goes after it. The user put it there.

It carries `position: last` because every goal on the chain is pinned there.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands what
makes it untrue, and not before:

- **By Stage 2's record**, because they state decisions this goal reverses:
  `docs/rules/core-classes/spreadsheet-bulk-boundary.md` (the cell cap, "reading pulls whole sheets",
  "a million-row export belongs in the queue"); `spreadsheet-evaluation-and-roster.md` (CSV now reads
  and writes through the component and is parsed by `Core\Csv`; the second wave keeps ods writing and
  evaluation only); `spreadsheet-has-no-io.md` ("one first-party Tier 1 extension package",
  "`nvs/spreadsheet`", "M17 builds it" — it is built in, and reading pulls byte ranges the host serves
  from what the program passed); `docs/rules/core-classes/decompression-bound.md` (a streamed part
  that is never held is bounded by the ratio alone); `docs/plan/m17.md` Part 3 (now this goal's, through
  a `## milestone: M17` wrap section).
- **By the stage that lands the behaviour:** every "Not shipped" paragraph of the five
  `spreadsheet-*` fragments; `docs/rules/packaging/the-first-party-components-are-built-in.md`'s count
  of three, at Stage 3.

The search that closes the stage, run after Stage 14:
`grep -rn -i "spreadsheet\|xlsx\|cell cap\|nvs/spreadsheet" docs/rules docs/reference docs/plan/m17.md`,
read line by line. Every hit is true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated and
are regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goals `ext-image`,
`ext-image-analysis`, `ext-intl`, `ldap` and `pdf` turned green stays green. `Core\Zip` and
`Core\Compress` keep refusing every bomb they refuse today: Stage 11's change to the decompression
bound applies only to a consumer that streams a part and never holds it.

## Stage 2 — the record, its fragments and `wit/spreadsheet.wit`

**Does:** Writes the `Novis\Spreadsheet` record from § *Standing decisions*, the fragments it creates
and rewrites, and the WIT interface the component exports.

One file set: `docs/decisions/`, `docs/rules/core-classes/`, `docs/rules/packaging/`,
`wit/spreadsheet.wit` (new).

- **The record**, this goal's one slot, written first. Every class, member and options shape under
  `Novis\Spreadsheet`; the write type table and the read type table of § *Standing decisions*; the
  style fields and their layering; the defaults; the template syntax; the formula helpers; the reader
  options and defaults; the limits; the error kinds; the tradeoffs; and the user's calls as decided. It
  amends ADR 0123 (built in, not a package; CSV through the component; the cell cap dropped; the
  PhpSpreadsheet concept-mapping table it owed is dropped, because goal `php-oracle-retired` removed
  every PHP-to-Novis table) and ADR 0166 (the streamed-part bound), and the built-in count of ADR 0247.
- **The fragments.** It rewrites the five `spreadsheet-*` fragments and `decompression-bound` to the new
  decisions, and creates at least these, under the ids the Stage 2 check reads:
  `core-classes/spreadsheet-writes-as-a-stream` (target first, batches, `finish`, sheets one after
  another, what stays changeable), `core-classes/spreadsheet-value-types` (both tables and the zone
  rule), `core-classes/spreadsheet-styles-and-defaults`, `core-classes/spreadsheet-template-fill`,
  `core-classes/spreadsheet-reads-as-a-stream`.
- **`wit/spreadsheet.wit`**, resource-shaped where intl is batch-shaped: a `writer` resource (`open`
  with the workbook options, `sheet`, `rows` taking a batch and returning the compressed bytes ready to
  leave, the sheet-end calls, `finish` returning the last bytes), a `reader` resource over a byte
  source, and a `template` resource. **How the guest reads byte ranges** is the record's call between
  two shapes: a host import serving ranges of the handle the program passed (it grants nothing, but
  it changes the `extension` world, so `crates/nvs-stdlib/tests/ext_world.rs` and
  `rule:packaging/a-guest-has-no-ambient-authority` change with it), or a pull loop in the Novis half
  where the guest returns "need bytes at offset N, length L" and the Novis code answers. The record
  says which, and why, against request latency.
- **The proofs checks the record makes possible**: one `bun nv proofs --verify --group '<class>'`
  check per class it names, added to Stage 14 of `data/goals/spreadsheet.json`, written here for the
  reason goal `pdf` gives (`gatesInFrontOfGroups` in `tools/nv/lib/chain.ts`).
- **A `cargo deny` over the component's own graph**, using the check goal `pdf` added.
- **Pinned by** the Stage 2 check.

## Stage 3 — the component is built in, and the streaming writer

**Does:** Builds `extensions/spreadsheet` into every binary as the fourth component, and writes a
typed sheet as a stream to every target in constant memory.

One file set: `extensions/spreadsheet/` (new), `crates/nvs-ext/build.rs:115` (`COMPONENTS`),
`crates/nvs-ext/src/builtin.rs:18`, `crates/nvs-ext/src/load.rs:667`,
`crates/nvs-ext/tests/spreadsheet.rs` (new),
`docs/rules/packaging/the-first-party-components-are-built-in.md`.

- **The crate** in `extensions/image`'s shape: its own `[workspace]`, `cdylib` and `rlib`, release
  with LTO and `panic = "abort"`, an empty WASI and no grant.
- **The writer is ours**, not `rust_xlsxwriter`: that crate's constant-memory mode writes through a
  `std::fs::File` per sheet (`src/worksheet.rs:1641` upstream), which a guest with no preopen cannot
  open. Our writer builds sheet XML as plain text, deflates it as it goes, and frames zip entries with
  data descriptors (`zip`'s `ZipWriter::new_stream`, or by hand with `miniz_oxide` and `crc32fast`).
  Strings are written inline (`t="inlineStr"`), so no string table grows. A batch of rows goes in, and
  the compressed bytes ready to leave come back.
- **The targets** (`rule:core-classes/spreadsheet-writes-as-a-stream`):
  `Workbook::createForDownload($name)` sets `Content-Disposition` and calls `Core\Response::stream` at
  once, `createForFile($path)` writes through `Core\IO::writeStream`, `createForStream($file)` writes
  to an open `Core\IO\File`, and `createInMemory()` keeps the bytes for `finish()`'s result. `finish()`
  writes the sheet-end parts, `styles.xml`, `workbook.xml`, the content types and the zip directory,
  and returns the row count per sheet and the size.
- **A workbook never finished** is logged as an error naming it when the program ends, and a download
  is cut off so the browser reports a failed download, not a broken file. Whether the server can
  abort a started stream that way was not checked when the goal was written. If it cannot, the
  session records the gap under `data/gaps/` naming `nvs-server` and keeps the log line.
- **The write type table** of § *Standing decisions*, with the `zone` rule. An `Instant` with no
  workbook `zone` throws naming the column.
- **Determinism** (`rule:core-classes/spreadsheet-output-is-inert`): fixed archive times, fixed
  document dates unless the program sets them, no map-order output. Two writes of equal input are
  byte-equal.
- **Pinned by** the Stage 3 checks.

## Stage 4 — columns, styles, defaults and sheets

**Does:** Adds columns, `Style` and its layering, the defaults for a sheet with a header, several sheets,
and the row limit.

One file set: `extensions/spreadsheet/src/write/` (styles, columns, sheet), `extensions/spreadsheet/nvs/`.

- **`columns`** keyed by the row's array keys: `header`, `width` (a number or `'auto'`), `format`,
  `style`, `hidden`, `formula`. A list of plain lists has no header.
- **`Style::of({...})`** with the fields of § *Standing decisions*. Column, then row, then cell: each
  field from the most specific layer that sets it. Every distinct combination becomes one entry of
  `styles.xml` at `finish()`; Excel's 65,490-format limit throws naming it. `format` takes a format code
  or the shortcuts `'date'`, `'datetime'`, `'time'`, `'duration'`, `'integer'`, `'decimal'`,
  `'money'`, `'percent'` and `'text'`.
- **The defaults** whenever a sheet has a header row, in `fromRows` and `sheet()` alike: the header
  bold with a thin bottom border, frozen, with an autofilter, and every column `width: 'auto'`. Each
  can be turned off (`{freeze: false, autofilter: false, headerStyle: null}`).
- **Auto width** measures the header and the rows held before the first chunk leaves (the first
  1,000), or every row when `fromRows` is given an array. Width counts characters as displayed
  through Stage 8's renderer, wide East Asian characters as two, capped at 60. No font metrics.
- **Sheets one after another.** `$book->sheet('B')` finishes sheet A; writing to a finished sheet
  throws naming it. `{position: 1}` sets the tab order, which `workbook.xml` records at `finish()`.
  Sheet names follow Excel's rules (31 characters, none of `: \ / ? * [ ]`, not empty, no leading or
  trailing apostrophe, unique), and a bad one throws naming the rule.
- **Row 1,048,577** throws naming the limit; `{overflow: 'newSheet'}` starts `Orders (2)` with the
  header repeated. A value past 32,767 characters throws naming the cell.
- **Pinned by** the Stage 4 checks.

## Stage 5 — the sheet-end parts, formulas, tables, conditional formats and images

**Does:** Adds everything xlsx writes after the rows or in its own part, so it may point at any earlier
row until the sheet is finished.

One file set: `extensions/spreadsheet/src/write/` (sheet tail, drawing, table), `extensions/spreadsheet/nvs/`.

- **Always in:** merged cells, hyperlinks (URL, `mailto:`, another sheet), notes, hidden rows,
  columns and sheets, outline grouping, row heights, document properties, page setup (orientation,
  fit to page, margins, print header and footer, repeated title rows, print area), data validation
  (list, number and date ranges, custom formula, input and error message), sheet protection (it locks
  the layout and is not security, which its `about.md` says), defined names, rich text in one cell, tab
  colour, zoom, hidden gridlines and right-to-left.
- **Formulas** (`rule:core-classes/spreadsheet-formula-is-a-value`): `Formula::of($text, cached: ...)`,
  the workbook flagged to recalculate when opened, `$sheet->cell($key, $row)` and
  `$sheet->column($key)` giving A1 text (`column` covers the data rows written so far), and a column's
  `formula: fn(int $row) => Formula`. No placeholder syntax inside formula text.
- **Excel tables:** style, banded rows, filter buttons, an optional totals row, and structured
  references (`[@qty]`) in the table's own formulas.
- **Conditional formats:** cell-value and formula rules, colour scales, data bars, icon sets, top and
  bottom N, duplicates and unique values, each rule's style a `Style` (font, fill, border, format).
- **Images:** PNG, JPEG and GIF bytes the program supplies, anchored to a cell with an optional size
  and offset. The image is its own zip entry, written as soon as it is added. The format is read from
  the bytes, and anything else throws.
- **Pinned by** the Stage 5 checks.

## Stage 6 — charts

**Does:** Adds the chart subset: column, bar, line, area, pie, doughnut, scatter and a combination of
two types, with titles, axis titles, the legend, series colours and data labels.

One file set: `extensions/spreadsheet/src/write/chart.rs` (new), `extensions/spreadsheet/nvs/Chart.nvs`
(new).

- **The chart XML** follows `rust_xlsxwriter`'s output for the same chart (MIT/Apache-2.0), ported
  for the subset only, with attribution in the module doc. A series names its ranges with
  `$sheet->column($key)`, so a chart over a stream points at every row the sheet ends with.
- **Out:** every other chart type, trendlines, error bars, secondary axes beyond the combination,
  chartsheets, and 3D.
- **Pinned by** the Stage 6 checks.

## Stage 7 — the reader

**Does:** Reads xlsx and xlsb as a stream over byte ranges, and xlsm, xls and ods whole, into the read
type table, with the header, row-number keys and the defaults of § *Standing decisions*.

One file set: `extensions/spreadsheet/src/read/` (new), `extensions/spreadsheet/nvs/Reader.nvs` (new),
`crates/nvs-ext/tests/spreadsheet_read.rs` (new).

- **Input** is an open `Core\IO\File` or `bytes`. The 100 MB never enters the guest: the guest reads
  the zip directory and each part through Stage 2's byte-range source, with a read-ahead buffer.
- **calamine** (0.36 when the goal was written) reads cells through `worksheet_cells_reader`. Its
  shared strings are one `String` per entry (`src/xlsx/mod.rs:256` upstream), and it exposes no cell
  styles. The reader keeps shared strings in **one arena with `u32` offsets**, and reads `styles.xml`'s
  number formats and each cell's `s=` index for date detection and for Stage 8. If calamine cannot be
  wrapped to do both, the session writes a thin reader over `zip` and `quick-xml` for xlsx, keeps
  calamine for xlsb, xls and ods, and says so in the module doc.
- **xls and ods** are read whole, as calamine does; an xls holds at most 65,536 rows. **xlsm** reads
  its data, and its macros are listed and never run.
- **`rows($sheet, {...})`**: `header` (`true`, or the row number of the header), `from`, `limit`,
  `columns` (`['A:C', 'F']`), `zone`, `text`, `formulas`, `emptyRows: 'keep'`, `merged: 'fill'`,
  `hidden: 'skip'`. The `foreach` key is the row number in the file. A duplicate header gets
  ` (2)`, an empty header cell becomes its column letter, and every row has every header key, with
  `null` for a missing cell.
- **`sheets()`** gives name, position, hidden or very hidden, size and kind. The extras read on
  request: merged ranges, hyperlinks, notes, defined names, tables, images as bytes, and document
  properties. An external link, a linked image or a data connection reads as data and is never
  followed (`rule:core-classes/spreadsheet-has-no-io`).
- **`tainted`**: every cell read from `tainted` bytes is `tainted`.
- **Pinned by** the Stage 7 checks.

## Stage 8 — the number-format renderer

**Does:** Renders a cell the way Excel displays it, for `{text: [...]}` on read and for auto width on
write.

One file set: `extensions/spreadsheet/src/format/` (new).

- **The format language**: up to four sections, conditions and colours (colours parsed and dropped),
  `0` `#` `?`, thousands separators and scaling, percent, scientific, fractions, literal text and
  escapes, `@`, and dates, times and elapsed time (`[h]`, `[mm]`), both epochs and the 1900 leap-day
  bug. The built-in format ids 0 to 49 map to their codes.
- **Separators** are those of `{locale}`, default `'en'`, read from `Novis\Intl`'s CLDR data. A format's
  `[$-407]` locale prefix is parsed and its separators are used.
- **The reference** is a table of format code, value and Excel's displayed text, committed as a
  fixture. SheetJS's SSF tests (Apache-2.0) are the source of cases.
- **Pinned by** the Stage 8 checks.

## Stage 9 — CSV through the same API

**Does:** `Reader` recognises CSV and passes it to `Core\Csv`, and `Workbook` writes CSV when the name
ends in `.csv` or `{format: 'csv'}` is given.

One file set: `extensions/spreadsheet/nvs/` (Reader, Workbook), `crates/nvs-stdlib/src/csv.rs` only if
`Core\Csv` lacks a piece the delegation needs.

- **Reading**: the same options and the same row shape as xlsx. `Core\Csv::rows` gives a short record
  fewer keys (`csv.rs:77`); the Novis half fills the missing header keys with `null`, so a row from
  either format has every key. The types are text, as CSV has no others.
- **Writing**: each batch is formatted by `Core\Csv::format` and leaves like an xlsx chunk. Styles,
  widths and every sheet-end part are ignored; a second sheet throws naming the format.
- **Pinned by** the Stage 9 checks.

## Stage 10 — template fill

**Does:** `Template::open($file)->fill($data)` copies every part of a designed workbook unchanged
except the cells it fills, repeats the rows marked by a list, and keeps every reference below them
right.

One file set: `extensions/spreadsheet/src/template/` (new), `extensions/spreadsheet/nvs/Template.nvs`
(new).

- **Placeholders**: `{{name}}` and `{{customer.name}}`. A cell that is only a placeholder keeps the
  value's type; text around it makes the cell text. A string beginning `=` stays text. A placeholder
  the data lacks throws naming it and its cell.
- **Repeated rows**: a row whose placeholder goes through a list (`{{items.qty}}`) is written once per
  item, with its style. A block of several rows repeats when it carries a defined name equal to the
  list's name. A list inside a list is out.
- **The shift**: rows below a repeated block move down, with their merges, formulas, defined names,
  print area, autofilter, data validation, conditional formats, tables, drawings' anchors and chart
  ranges. A formula or chart range that covers the template row grows with the block (`SUM(C5:C5)`
  becomes `SUM(C5:C104)`). The shifter tokenizes A1 references (relative, absolute, ranges, whole
  rows and columns, other sheets) and leaves everything else in the formula text alone.
- **`->set('Summary!B2', $value)`** sets one cell by address.
- **Output** is any of the targets: `->download($name)`, `->save($path)`, `->to($file)`, `->bytes()`.
  An xlsm template gives xlsx with the macros dropped. Memory is the template plus one batch: the
  repeated rows stream.
- **Pinned by** the Stage 10 checks.

## Stage 11 — limits and hostile files

**Does:** Implements the decompression bound for streamed parts and proves every hostile-file class of
the genre ends in a throw or a contained trap.

One file set: `crates/nvs-stdlib/src/zip.rs`, `docs/rules/core-classes/decompression-bound.md`,
`extensions/spreadsheet/src/read/`, `tests/hostile/novis/` (the spreadsheet attacks).

- **The bound** (`rule:core-classes/decompression-bound`, as Stage 2 rewrites it): a part held whole
  stays under `min(input × ratio, max_decompressed)`; a part streamed and never held is bounded by
  `input × ratio` alone, and the archive-wide total counts held parts only. The host passes the
  configured values to the guest the way `[image] max_pixels` is clamped before a call
  (`crates/nvs-ext/src/builtin.rs:35-58`). `Core\Zip` keeps today's behaviour for every caller that holds
  its output.
- **No new limit.** Row and column indexes past Excel's 1,048,576 × 16,384 throw naming the cell. The
  XML parser resolves no entity and loads no DTD. The xls reader refuses a sector chain that visits a
  sector twice; if calamine lacks the check, the session adds it in our wrapper or upstream.
- **The corpus**: a zip bomb in a sheet and in `sharedStrings.xml`, a billion-laughs entity, an
  external entity naming a file and an `http://169.254.169.254/` URL, a row index of 2^31, a merged
  range past the grid, an xls sector loop, an xlsb record whose length lies, a template whose
  repeated row names a list of 10 million items under a small memory cap, and an export string
  beginning `=HYPERLINK(...)`.
- **Pinned by** the Stage 11 checks.

## Stage 12 — 100 MB, the growth and the benches

**Does:** Proves a 100 MB workbook writes and reads in memory proportional to rows in flight, records
what the component adds to the binary, and commits the benches.

One file set: `extensions/spreadsheet/src/lib.rs` (its module doc), `crates/nvs-ext/tests/builtin.rs`,
`benches/members/novis/`.

- **The big file** is generated by the test, never committed: about 100 MB of xlsx, numbers, dates
  and text in 20 columns. It is written to a file and read back. The guest's peak memory is recorded,
  and the growth check holds it flat as rows grow (`rule:testing/feature-proofs`' growth). A
  text-heavy file written by another tool, with a large shared string table, is measured too, and its
  peak is the size of its strings plus a batch.
- **The growth** of the binary is recorded in the component's module doc and held by
  `the_recorded_growth_matches_the_embedded_components`. The goal writer's estimate was 1 to 3 MB, not
  checked. If it looks too large, the figure goes into the handoff's `## Backlog`; nothing becomes
  optional, which is the user's call.
- **Benches**: a million-row export, a million-row import, a styled report with a chart, and a
  template filled with 10,000 rows, each with its growth.
- **Pinned by** the Stage 12 checks.

## Stage 13 — what opens in Excel

**Does:** Checks that what the writer and the template filler produce is what Excel itself writes.

One file set: `extensions/spreadsheet/tests/` (the reference comparisons), `extensions/spreadsheet/fixtures/`.

- **Round trip**: every write fixture is read back by our reader and by calamine directly, and the
  values, types, merges, links, notes and validations match what was written.
- **Against Excel's own files**: `rust_xlsxwriter`'s test suite compares against files Excel created
  (MIT/Apache-2.0). The fixtures for styles, tables, conditional formats, data validation, images and
  every chart type in the subset are copied with attribution, and each part our writer produces is
  compared as XML with the same normalisation that suite uses. A difference is fixed in the writer,
  never in the fixture.
- **Pinned by** the Stage 13 checks.

## Stage 14 — the feature proofs

**Does:** Writes the feature proofs for every member the record named.

One file set: `docs/examples/novis/`, `tests/hostile/novis/`, `benches/members/novis/`,
`docs/reference/novis/` (new pages for `Novis\Spreadsheet`), `data/proofs/policy.json`.

- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench, one attack and its help in the binary. `bun nv proofs --id '<member>'`
  prints what is owed.
- **The manifest class** follows `Novis\Image\Codec`'s rule: tests, attack and help in full;
  examples and bench in `data/proofs/policy.json`'s `skip`, one reason each.
- **The examples** are the cases users meet: an orders export as a download, a report with totals and
  a chart, an import of an upload with errors reported by row number, a zip-code column read as text,
  a monthly report from a template with an item list, a two-sheet export with the summary first, and
  the same export as CSV.
- **Pinned by** the Stage 14 checks.

## Standing decisions

- **The user's calls, as instructions.** One goal. `Novis\Spreadsheet` is the fourth built-in
  component, always on, under `Novis\`. **The component never touches disk**: writing returns
  compressed chunks the Novis half sends on, reading pulls byte ranges of a file the program opened
  or bytes it holds, and no preopen is granted. **Streaming is the only way to write**: a row's values
  and styles are final once written; the sheet-end parts may point at any earlier row until the sheet
  is finished; frozen panes and widths are set before the first row; there is no in-memory editable
  mode. **Template fill is our own streaming filler**, with list placeholders marking repeated rows,
  a defined name for a block of several rows, and no list inside a list. **Reading gives typed values**
  (the read table below) and `{text: [...] | true}` gives Excel's displayed text. **A `zone` option** on
  the reader (default `"UTC"`, the clock time kept exactly) and the workbook (an `Instant` with no
  `zone` throws). **Styles are `Style::of({...})` values**, layered column, row, cell. **A sheet with a
  header row is styled by default**: bold header, bottom border, frozen, autofilter, auto width
  (header and the first 1,000 rows, capped at 60). **Sheets are written one after another**, with
  `{position}` for tab order. **Images, Excel tables, conditional formats and the chart subset are
  in.** **The target comes first**: `Workbook::createForDownload`, `createForFile`, `createForStream`,
  and `createInMemory`; the one-call helpers (`fromRows`, `Template::fill`) take the verb last
  (`->download()`, `->save()`, `->to()`, `->bytes()`). **The last call is `finish()`.** **Reading
  defaults**: empty rows skipped, a merged value in its top-left cell only, hidden rows read, the
  `foreach` key the row number. **CSV reads and writes through the same API** and is parsed and
  formatted by `Core\Csv`. **Writing ods is out** and stays in the rule's second wave with formula
  evaluation. **Formulas are plain Excel text**, with `$sheet->cell()`, `$sheet->column()` and a
  column's `formula: fn(int $row)`. **No new limit**, and the cell cap is dropped. **The decompression
  bound** treats a part that is streamed and never held by the ratio alone.
- **The write table**: `string` is a text cell, always; `int` and `float` are numbers; an `int` or
  `decimal` with more than 15 significant digits is a text cell, so no digit is lost; `decimal` is a
  number; `bool` is TRUE or FALSE; `Date`, `TimeOfDay` and `Duration` are serials with the formats
  `yyyy-mm-dd`, `hh:mm:ss` and `[h]:mm:ss`; `DateTime` is its own clock time with
  `yyyy-mm-dd hh:mm`; `Instant` is its clock time in the workbook's `zone`; `null` is an empty cell;
  `Formula` is a formula; `Cell::of($value, $style)` styles one cell.
- **The read table**: text is `string`; a whole number within ±2^53 is `int`, another number `float`;
  TRUE and FALSE are `bool`; a number with a date format is `DateTime` in `zone`, `Date` when the
  format has no time and `TimeOfDay` when it has only one; an elapsed-time format is `Duration`; an
  error is a `CellError` case; empty is `null`; a formula reads as its stored result, and
  `{formulas: true}` gives the `Formula` with its stored result inside.
- **The goal writer's calls, not confirmed by the user, also standing.** `createInMemory`, not
  `createForMemory`. `finish()` returns the row count per sheet and the size, and `->bytes` for
  `createInMemory`. A workbook never finished is logged and its download cut off. The header option is
  `header`, as `Core\Csv::rows` names it, and `true` is not the default, as in `Core\Csv`. The
  row-overflow default throws, and `{overflow: 'newSheet'}` continues. `{locale}` for the renderer
  defaults to `'en'`. A duplicate header gets ` (2)`, an empty one its column letter. The 1,000-row
  auto-width window and the cap of 60. The PhpSpreadsheet concept-mapping table ADR 0123 owed is
  dropped. Class names in the sketch and the checks (`Workbook`, `Sheet`, `Reader`, `Template`,
  `Style`, `Cell`, `Formula`, `CellError`, `Chart`, `Engine` for the manifest class) are the record's
  to change; a rename rewrites the checks in the same slice.
- **Out, for good unless the user reopens it**: pivot tables, sparklines, threaded comments, writing
  encrypted or password-protected files, writing xls, every chart type outside the subset, lists
  inside lists in templates, a per-cell accessor (`rule:core-classes/spreadsheet-bulk-boundary`), and
  a grant of any directory to the component.
- **One record slot**: one new record and no other number, checked against `docs/decisions/` right
  before it is written, because another agent may take a number first. It is Stage 2's first slice.
- **The crates**, each pinned: `calamine`, `zip` with `deflate` only, `miniz_oxide` or `flate2` on its
  pure-Rust backend, `crc32fast`, `quick-xml`, `itoa`, `ryu`. `rust_xlsxwriter` is a source of chart XML
  and of Excel-made fixtures, never a dependency. `umya-spreadsheet` and `ironcalc` are not
  dependencies. A crate that fails to build for `wasm32-wasip2` is replaced by its nearest pure-Rust
  equivalent, and the record says so.
- **How a session judges output**: values by reading back (Stage 13), parts by comparing XML with
  Excel-made fixtures, memory by the guest's peak in the growth check. A fixture is never rewritten
  to make a check pass.
- **Determinism is part of every stage**: two writes or two template fills of equal input are
  byte-equal.
- **The tradeoffs**, stated here and in the record because AGENTS.md asks. Performance: one guest call
  per batch, not per cell; a download starts before the first row is written. Memory: nothing until
  first use; then the compiled module, shared by every core, and per request one batch of rows, the
  compressor's window, the sheet-end parts and, on read, the shared strings of the file being read,
  all inside the request's cap and freed when the request ends. The binary grows by Stage 12's
  measured figure. Usability: an export in one line, styled by default, imports with typed values and
  row numbers, templates designed in Excel. Simplicity: one way to write, no editable mode, and
  forward-only rules that are written down once.
- **Neutral names only** in every test, example and record: `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ext` or a
  `--test` filter.
