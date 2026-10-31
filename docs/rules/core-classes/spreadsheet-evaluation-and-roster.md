The format roster is **read-wide and write-narrow**. xlsx reads and writes. CSV reads and writes through
the same classes, parsed and formatted by `Core\Csv`. xlsm reads its data, with its macros listed and
inert, and **is never written**: a filled xlsm template gives xlsx with no VBA part. xlsb and xls
(BIFF8) read and do not write, because a writer for a 1997 binary format is legacy nothing should
produce. ods reads and writes, from the same `Workbook` calls as xlsx, by our own writer, which holds
the workbook until `finish()` (`rule:core-classes/spreadsheet-writes-as-a-stream`). Template fill is in
the first wave (`rule:core-classes/spreadsheet-template-fill`), because it is the workflow behind most
real exports.

**The crates**, each pinned and pure Rust, built for `wasm32-wasip2`, on `deny.toml`'s allowlist and
with no C in the build. Every format is read by our own reader. For xlsx and xlsb, `calamine` keeps
one `String` per shared string, exposes no cell style, and owns the `Read + Seek` a reader resource
must keep its place in between calls. For xls and ods it is the crate that fails to build: its
release on a `quick-xml` with no open advisory needs a `zip` that pulls in `typed-path`, which does
not build for `wasm32-wasip2` on stable Rust, and its release that builds carries `quick-xml`'s two
open advisories, which `deny.toml` refuses. Its replacement is our reader of the compound file and
BIFF8 records for xls, and of `content.xml` for ods. `zip` with `deflate`
only, and `miniz_oxide` or `flate2` on its pure-Rust backend, frame and inflate the archive;
`crc32fast` checks it; `quick-xml` reads and writes the XML; `itoa` and `ryu` write numbers. The
writer, the template filler and the number-format renderer are our own. A crate that fails to build
for `wasm32-wasip2` is replaced by its nearest pure-Rust equivalent, and this fragment names the
replacement. `rust_xlsxwriter` is a source of chart XML and of Excel-made fixtures, never a
dependency; `umya-spreadsheet` and `ironcalc` are not dependencies.

**Nothing evaluates implicitly** — not on read, and not on write, where a formula cell is written with
its cached result when the program gave one and the workbook is flagged to recalculate when opened. A
read formula cell is its stored result, and `{formulas: true}` gives the `Formula` with that result as
its `cached`, so the program can tell the author's claim from the cell's text
(`rule:core-classes/spreadsheet-value-types`).

**`Reader::evaluate(): array<{cell: tainted string, kind: string, text: tainted string}>`** computes on
request. It computes every formula cell of every sheet once, in the guest, in dependency order, and
returns what it could not compute: `cell` as `Sheet!A1`, `kind` as `'function'` (a function outside
the set, with its name), `'reference'` (another workbook, a missing sheet, a 3D reference, a union, an
intersection, an array constant, or a result of more than one cell, with the reference), `'circular'`
(with the formula) or `'token'` (formula text the evaluator cannot read, or a formula the reader could
not decode). A test asserts that list is empty, the visibility rule the PDF component applies to
dropped CSS (`rule:core-classes/pdf-css-subset`). After it, each `rows()` reads a formula cell as its
computed result, through the read table and the cell's number format, and `formulas: true` gives that
result as the `cached`. A cell it did not compute keeps its stored result, and a cell that reads it
reads that result. Every format feeds it: xlsx formula text (a shared formula moved to each cell),
xls and xlsb tokens through the token decoder, and ods OpenFormula through the translator. A CSV has
no formulas and returns `[]`.

- **Security.** It does no I/O and follows nothing. An external reference, `INDIRECT` and `OFFSET`
  are listed. It runs under the request's CPU and memory caps. A chain of 20,000 cells computes, and
  a cycle of any length is listed, because the order is walked with an explicit stack.
- **Memory.** One value per non-empty cell of every worksheet, in a sparse grid, so `A:A` walks only
  stored cells; the parsed formulas and the order. Per request, freed with the reader.
- **Latency.** One guest call, growing with the cells and the references. No existing read path
  pays anything until a program calls it.
- **Excel's behaviour.** Numbers are `f64`, and coercion and error values follow Excel. Text
  converts to a number when it is a number, a percentage, an ISO date or a clock time, and is
  `#VALUE!` otherwise, so a date in a locale's own order (`1/15/2024`) is `#VALUE!` here, because
  the result would depend on the machine. An unknown name is `#NAME?`. Defined names and structured
  references (`Table[Column]`, `[#This Row]`, `[#Data]`, `[#All]`, `[#Headers]`, `[#Totals]`)
  resolve, and a sheet-qualified name reads only that sheet's names.
- **Bounds.** Parentheses and calls more than 64 deep, a tree more than 512 nodes deep, or more
  than 4,096 operators and calls in one formula are `token`. A name chain more than 64 deep is
  `circular`. `ROW`, `COLUMN`, `ROWS` and `COLUMNS` read no cells, so `=ROWS(A1:A9)` in `A1` is not
  circular. `IF`, `IFERROR`, `IFNA`, `IFS` and `SWITCH` evaluate only what they take, so a function
  outside the set in a branch not taken is not listed. `SUBTOTAL` with a code from 1 to 11 over a
  range with a hidden row is listed, because the file does not say whether a filter hid it.

**The function set** is one table, `extensions/spreadsheet/src/eval/functions.rs`, and a test gives
each function a case:
- Math and statistics: `ABS`, `AVERAGE`, `AVERAGEIF`, `AVERAGEIFS`, `CEILING`, `CEILING.MATH`,
  `COUNT`, `COUNTA`, `COUNTBLANK`, `COUNTIF`, `COUNTIFS`, `EXP`, `FLOOR`, `FLOOR.MATH`, `INT`, `LN`,
  `LOG`, `LOG10`, `MAX`, `MAXIFS`, `MEDIAN`, `MIN`, `MINIFS`, `MOD`, `PI`, `POWER`, `PRODUCT`,
  `ROUND`, `ROUNDDOWN`, `ROUNDUP`, `SIGN`, `SQRT`, `SUBTOTAL` (codes 1 to 11 and 101 to 111), `SUM`,
  `SUMIF`, `SUMIFS`, `SUMPRODUCT`, `TRUNC`.
- Logic: `AND`, `FALSE`, `IF`, `IFERROR`, `IFNA`, `IFS`, `NOT`, `OR`, `SWITCH`, `TRUE`, `XOR`.
- Information: `ISBLANK`, `ISERR`, `ISERROR`, `ISEVEN`, `ISLOGICAL`, `ISNA`, `ISNUMBER`, `ISODD`,
  `ISTEXT`, `NA`.
- Text: `CHAR`, `CLEAN`, `CODE`, `CONCAT`, `CONCATENATE`, `EXACT`, `FIND`, `LEFT`, `LEN`, `LOWER`,
  `MID`, `PROPER`, `REPLACE`, `REPT`, `RIGHT`, `SEARCH`, `SUBSTITUTE`, `TEXT` (through the
  number-format renderer), `TEXTJOIN`, `TRIM`, `UPPER`, `VALUE`. Lengths and positions count UTF-16
  units, as Excel's do.
- Lookup: `CHOOSE`, `COLUMN`, `COLUMNS`, `HLOOKUP`, `INDEX`, `MATCH`, `ROW`, `ROWS`, `VLOOKUP`,
  `XLOOKUP`.
- Date and time: `DATE`, `DATEDIF`, `DAY`, `DAYS`, `EDATE`, `EOMONTH`, `HOUR`, `MINUTE`, `MONTH`,
  `SECOND`, `TIME`, `WEEKDAY`, `YEAR`.

`TODAY`, `NOW`, `RAND` and `RANDBETWEEN` are out, because the result would depend on when or how often
the program reads. `INDIRECT` and `OFFSET` are out, because they make a reference from data. These and
every function outside the table are listed. A function of the set given an input on which it cannot
match Excel is listed for that cell, and never computed differently.

**On disk.** Every format the roster reads (`extensions/spreadsheet/src/read/`) and writes
(`extensions/spreadsheet/src/writer.rs`, `extensions/spreadsheet/src/ods/`, and CSV in
`extensions/spreadsheet/nvs/`). The evaluator is `extensions/spreadsheet/src/eval/`, fed by
`extensions/spreadsheet/src/read/evaluate.rs`.

**Not shipped.** An ods's sheet-end parts and charts: `Sheet`'s `merge`, `link`, `note` and the other
parts throw for an ods (gap:nvs-spreadsheet/spreadsheet-evaluate-and-write-ods).

**Declined:**
- **Evaluating formulas on read.** Recalculating on open turns reading an attacker's upload into
  running the attacker's program (`rule:core-classes/spreadsheet-has-no-io`).
- **An option to compute `TODAY` and `NOW` from a given instant.** A program that needs one fills a
  cell with the instant.
- **Writing xls, xlsb or xlsm.** Legacy and binary formats nothing should produce, and a macro-enabled
  format whose only purpose over xlsx is the macros.
- **`umya-spreadsheet` for template fill.** It loads the whole workbook into an editable model
  (`rule:core-classes/spreadsheet-template-fill`).
