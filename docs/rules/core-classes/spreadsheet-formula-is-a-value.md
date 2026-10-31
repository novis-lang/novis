Only an explicit `Formula` value produces a formula cell. Every string writes as a **text cell** —
including one beginning `=`, `+`, `-`, `@` or a control character, which is the whole trigger set for
the formula-injection class.

An export built from user data is therefore inert by construction rather than by a caller remembering
to prefix a quote. This is the same move taint tracking makes everywhere else — code and data
separated by type, not by inspection — applied at a boundary that is otherwise a footnote in a
security advisory. Because the writer cannot be tricked into emitting code, it needs no launderer. A
template fill holds to the same rule: a value beginning `=` fills as text
(`rule:core-classes/spreadsheet-template-fill`).

**A formula is plain Excel text**, `Formula::of('SUM(C2:C9)', {cached: 42})`, with no placeholder syntax
inside it. Three helpers write the references, so a program never computes a column letter:

- `$sheet->cell($key, $row)` is the A1 address of the column `$key` (a column key or a letter) in row
  `$row`, such as `C7`.
- `$sheet->column($key)` is the column's data range over the rows written so far, such as `C2:C1043`,
  so a totals row written after the data covers every row of the stream.
- A column's `formula: fn(int $row) => Formula` is called once per row with that row's number, and its
  `Formula` is the column's cell in that row.

**A formula has 1 to 8,192 characters** once its leading `=` is dropped, Excel's limit. One check
(`cells::check_formula`) holds every formula the writer writes: a `Formula` cell, a column's formula,
a table total, a conditional format and a validation. `Formula::of('')` throws `LogicError` in the
Novis half, before any row is written.

`cached` is the result the file carries beside the formula, for a reader that does not recalculate;
with none, the cell has no cached value. Every written workbook is flagged to recalculate when it is
opened, so Excel shows computed values whatever was cached.

What it costs is one wrapper at the call sites that genuinely mean a formula, which is the smaller
half of any real export.

The component's tests owe a `tainted` string beginning `=` that lands as a text cell, and the same
expression as a `Formula` value that lands as a formula cell, both asserted on the written bytes
rather than the builder's state.

**On disk.** `Formula`, the three helpers and the text-cell rule
(`extensions/spreadsheet/nvs/Formula.nvs`, `Sheet.nvs`).

**Declined:**
- **Placeholders inside formula text, such as `{qty}*{price}`.** It is a second syntax inside Excel's,
  and the helpers give the same references as Novis strings.
- **A string beginning `=` written as a formula, with an escape for text.** Every export of user data
  would then be one forgotten escape from injection.
