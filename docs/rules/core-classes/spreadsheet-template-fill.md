`Template::open($source)->fill($data)` copies every part of a designed workbook unchanged except the
cells it fills, writes a row marked by a list once per item, and moves every reference below it so the
formulas, tables and charts still point at the right rows.

**Placeholders** are `{{name}}` and `{{customer.name}}`, a path into `$data`'s keys. A cell that is
only a placeholder takes the value's cell type by `rule:core-classes/spreadsheet-value-types`'s write
table and keeps the cell's style; a placeholder with text around it makes the cell text. A string
value beginning `=` stays text (`rule:core-classes/spreadsheet-formula-is-a-value`). A placeholder
the data lacks throws `LogicError` naming it and its cell. Formula text holds no placeholder: a
formula refers to the cells that do.

**Repeated rows.** A row whose placeholder goes through a list, `{{items.qty}}`, is written once per
item of `$data['items']`, each copy with the template row's style and height. A block of several rows
repeats as one when a defined name equal to the list's name covers it, so an item can span two rows.
**A list inside a list throws**, naming both. An item whose copy would end past row 1,048,576 throws
`LogicError` naming the list, before any of its rows is written.

**Lists on several sheets.** Each sheet may repeat its own lists, and the sheets with lists are
written in workbook order, each with its own shifts. The lists are filled in the order their blocks
stand: sheet order, then row. A block's shift moves every reference to its rows, from its own sheet
and from every other sheet and part. A sheet waits for its items, and is written when the fill
finishes, when a formula above one of its blocks names a row that block moves, or when a formula or
part on it names a row a list on a later sheet moves. It then costs the memory of its items.

**The shift.** Every row below a repeated block moves down by the rows it added, and every part that
points at those rows moves with it: merged ranges, formulas, defined names, the print area, the
autofilter, data validation, conditional formats, tables, drawing anchors and chart ranges. A range
that covers the template row grows with the block, so `SUM(C5:C5)` over a block of 100 items becomes
`SUM(C5:C104)`. The shifter tokenizes A1 references — relative, absolute, ranges, whole rows and
columns, and references to other sheets — and leaves every other character of formula text as it was.

**`->set('Summary!B2', $value)`** fills one cell by address, by the same write table, before `fill`.
`fill` returns an `Export`, so the target is the last call: `->download($name)`, `->save($path)`,
`->to($file)` or `->bytes()`. The source is a `Core\IO\File` or `bytes`, read as
`rule:core-classes/spreadsheet-reads-as-a-stream` reads one. **The source is never `tainted`:** the
filled file carries the template's own formulas, links and sheets unescaped, so filling is safe only
with a template the program trusts. A program filling an upload says so with
`Core\Taint::assertTrusted($bytes, $reason)` itself, and with a plain source no taint reaches the
written file, so the `Export` stays plain `bytes`. **An xlsm template gives xlsx**: the
`vbaProject` part and its relationship are dropped, and the content type is the xlsx one
(`rule:core-classes/spreadsheet-output-is-inert`). Two fills of equal input are byte-equal.

The tradeoffs. **Usability:** a designer builds the invoice in Excel, and the program supplies data.
**Simplicity:** one placeholder syntax, one list rule and no list inside a list. **Performance and
memory:** the repeated rows stream, so a template of a thousand items costs what one batch does.

- **Memory:** per filling request, the parts the template carries that are not sheet rows (the styles,
  the drawings, the parts the shift rewrites), the template's sheet rows as they stream through, and
  one batch of repeated rows, freed when the request ends.

**On disk.** `Template::open`, `set` and `fill`, and an `Export` from `fill` to all four targets
(`extensions/spreadsheet/nvs/Template.nvs`, `extensions/spreadsheet/src/template/`).

**Not shipped.** A
shared formula (`<f t="shared">`) in a repeated row is copied as it is, so its copies share one
`si`, and an `x14` conditional format under `extLst` is not moved. A list is an array: an
`Iterator` in `$data` is not a list (gap:nvs-spreadsheet/spreadsheet-template-fill-limits).

**Declined:**
- **A list inside a list.** It needs nested blocks whose shifts compound, for a layout almost nobody
  designs; a program that needs it writes the sheet with `Workbook`.
- **Placeholders inside formula text.** A formula that names a filled cell does the same job, and
  text substituted into a formula is the injection the formula rule closes.
- **`umya-spreadsheet` as the filler.** It loads the whole workbook into an editable model, which is
  the memory a template with a long list must not cost.
- **`Template::open` taking `tainted bytes`.** An uploaded template's formulas and external links
  reach the filled file as written, and taint flowing through the engine would make every export
  `tainted bytes`; the one explicit `assertTrusted` at the upload is the honest place for the trust.
- **Keeping the macros of an xlsm template.** The writer emits no executable content
  (`rule:core-classes/spreadsheet-output-is-inert`).
