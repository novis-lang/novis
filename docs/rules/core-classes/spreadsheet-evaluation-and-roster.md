The format roster is **read-wide and write-narrow**. xlsx reads and writes. CSV reads and writes through
the same classes, parsed and formatted by `Core\Csv`. xlsm reads its data, with its macros listed and
inert, and **is never written**: a filled xlsm template gives xlsx with no VBA part. xlsb and xls
(BIFF8) read and do not write, because a writer for a 1997 binary format is legacy nothing should
produce. ods reads, and writing it is the second wave. Template fill is in the first wave
(`rule:core-classes/spreadsheet-template-fill`), because it is the workflow behind most real exports.

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
(`rule:core-classes/spreadsheet-value-types`). An explicit evaluate call is second-wave work, and so is
writing ods; neither is in the first wave's acceptance. An evaluate call would return the list of
functions and references it could not compute, so a test asserts that list is empty — the visibility
rule the PDF component applies to dropped CSS (`rule:core-classes/pdf-css-subset`).

**On disk.** Every format the roster reads (`extensions/spreadsheet/src/read/`) and writes
(`extensions/spreadsheet/src/writer.rs`, and CSV in `extensions/spreadsheet/nvs/`).

**Not shipped.** An evaluate call and writing ods (gap:nvs-spreadsheet/spreadsheet-evaluate-and-write-ods).

**Declined:**
- **Evaluating formulas on read.** Recalculating on open turns reading an attacker's upload into
  running the attacker's program (`rule:core-classes/spreadsheet-has-no-io`).
- **Writing xls, xlsb or xlsm.** Legacy and binary formats nothing should produce, and a macro-enabled
  format whose only purpose over xlsx is the macros.
- **`umya-spreadsheet` for template fill.** It loads the whole workbook into an editable model
  (`rule:core-classes/spreadsheet-template-fill`).
