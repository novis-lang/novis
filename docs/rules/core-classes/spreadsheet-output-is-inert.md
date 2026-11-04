The writer emits no macros, no embedded executables and no launch-shaped content; hyperlinks exist
only where the program wrote them.

**A reference the program writes stays in the workbook.** `Workbook::define` and a chart's series
range go through one reference parser (`extensions/spreadsheet/src/reference.rs`): an optional sheet
name, quoted as Excel quotes one, then `!`, then a cell, a range, whole columns (`$A:$C`) or whole
rows (`$1:$3`), each with or without `$`. A series range takes no whole columns or rows. The sheet
name follows the sheet name rule (`rule:core-classes/spreadsheet-bulk-boundary`). Another workbook
(`[other.xlsx]Shop!A1`), a DDE call (`cmd|'/c calc'!A0`), a URL, a function, an R1C1 reference and
a second sheet name throw `LogicError`, so neither DDE nor an external link reaches the file
through a name or a chart. A name whose sheet the workbook never adds throws at `finish()`, naming
the name and the sheet.

It embeds **no timestamps and no generator entropy** — document properties carry fixed dates unless
the program sets them, archive entries a fixed time, and no output depends on a map's order — so equal
input gives byte-equal output, for a write and for a template fill alike, and a document test is a
byte comparison. This is the same rule the PDF writer holds (`rule:core-classes/pdf-output-is-inert`),
and it exists for the same reason: a test that cannot compare bytes ends up comparing nothing.

**A template fill drops every part that can run code**, with its relationships and content-type
entries: the VBA project, XLM macro sheets, `xl/vbaData.xml`, ActiveX controls under `xl/activeX`,
`customUI` ribbons and `oleObject*.bin` embeddings (`extensions/spreadsheet/src/template/mod.rs`'s
`dropped`). A macro sheet's `<sheet>` leaves the workbook with the defined names local to it or
naming it, and `localSheetId`, `activeTab` and `firstSheet` move to the sheets that stay. A
worksheet's `control` and `oleObject` that name a dropped part leave the sheet. An embedded package,
such as an xlsx or a docx, and every image are kept: neither runs when the file opens. Cost: none
per cell; one pass over the content types, the relationships and the workbook part, which the fill
rewrites already.

The component's tests owe a filled `.xlsm` template whose output a scanner finds no `vbaProject`
part in, a filled template with a macro sheet, an ActiveX control, a ribbon and an OLE object whose
output has none of them and keeps its images, and the same input written twice and filled twice,
each pair compared byte for byte.

**On disk.** The writer (`extensions/spreadsheet/src/writer.rs`) and the template filler
(`extensions/spreadsheet/src/template/`) both hold the rule, and `extensions/spreadsheet/tests/excel.rs`
reads what each makes back through two readers. `extensions/spreadsheet/tests/write.rs` and
`chart.rs` hold the reference parser to its forms.

**Declined:**
- **A constant or a formula as a defined name's value**, such as `0.2` or `SUM(Shop!A1:A9)`. A
  name's value is the one place a formula would be written without `Formula`, and a formula there is
  where DDE and external links hide. `Formula` is the one way to write a formula, and a constant goes
  in a cell the name points to.
