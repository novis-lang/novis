The roster is **read-wide and write-narrow**. The modern XML format reads and writes, and is the
parity target: styles, merged cells, charts, conditional formats, tables, autofilters, images and
defined names. The macro-enabled variant reads data only, with its macros enumerable and inert, and
**is never written** — a filled template emits the macro-free format. The binary and legacy formats
read and do not write, because a writer for a 1997 binary format is legacy nothing should produce.
The open format reads, and writes in a second wave. Template fill is named explicitly, because it is
the workflow behind most real exports: open a styled workbook, set values, save — styling preserved,
macros dropped.

**Nothing evaluates implicitly** — not on read, and not on write, where a formula cell is written for
the opening application to compute. An explicit evaluate call is second-wave work, and its result
carries the list of functions and references it could not compute, so a test asserts that list is
empty. That is the same visibility rule the PDF component applies to dropped CSS: silent wrongness
becomes a named, testable diagnostic.

**Not shipped.** There is no spreadsheet package in the tree.
