The roster is **read-wide and write-narrow**. xlsx reads and writes, and is the parity target: styles,
merged cells, charts, conditional formats, tables, autofilters, images and defined names. xlsm reads
data only, with its macros enumerable and inert, and **is never written** — a filled template emits
xlsx with no VBA stream. xlsb and xls (BIFF8) read and do not write, because a writer for a 1997
binary format is legacy nothing should produce. ods reads, and writes in a second wave. Template fill
is named explicitly, because it is the workflow behind most real exports: open a styled workbook, set
values, save — styling preserved, macros dropped.

Reading is `calamine`, generation `rust_xlsxwriter`, template fill `umya-spreadsheet` and the
second-wave ods writer `spreadsheet-ods`: every crate pure Rust, wasm-clean and on `deny.toml`'s
allowlist, with no C in the build.

**Nothing evaluates implicitly** — not on read, and not on write, where a formula cell is written for
the opening application to compute. An explicit evaluate call is second-wave work over IronCalc, the
one credible Rust engine, and the wave waits on its 1.0 release: a version test rather than a
judgment. Its function set is documented, and its result carries the list of functions and references
it could not compute, so a test asserts that list is empty. That is the same visibility rule the PDF
component applies to dropped CSS: silent wrongness becomes a named, testable diagnostic. Until that
wave, a workbook's cached formula results are returned where PhpSpreadsheet would recalculate, and the
API tells the two apart, so the gap is visible.

**Not shipped.** There is no spreadsheet package in the tree.

**Declined:**
- **Evaluating formulas on read.** Recalculating on open turns reading an attacker's upload into
  running the attacker's program (`rule:core-classes/spreadsheet-has-no-io`).
