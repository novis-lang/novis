The parts xlsx writes after a sheet's rows or in a part of its own are `Sheet` calls that may point at
any row already written, until the sheet is finished: merged ranges, hyperlinks, notes, data
validation, outline groups, Excel tables, conditional formats, images, charts, page setup and
protection.

| Call | What it writes |
|---|---|
| `merge(string $range)` | a merged range |
| `link(string $cell, string $target, {text?, tooltip?})` | a hyperlink to a URL, a `mailto:` address or `'Sheet!A1'` |
| `note(string $cell, string $text, {author?})` | a note |
| `validate(string $range, {...})` | data validation: `list`, `whole`, `decimal`, `date` or `length` with `between`, `min` or `max`, or `formula`; `input` and `error` messages; `allowEmpty` |
| `group(string $rows, {collapsed?})` | an outline group over `'5:9'` or `'B:D'` |
| `table(string $range, {name?, style?, banded?, filter?, totals?})` | an Excel table; `totals` maps a column to `'sum'`, `'average'`, `'count'`, `'min'`, `'max'` or a `Formula`, and the table's own formulas may use structured references such as `[@qty]` |
| `conditional(string $range, {...})` | one conditional format: `cell` with an operator and values, `formula`, `colorScale`, `dataBar`, `iconSet`, `top` or `bottom` N, `duplicates` or `unique`, each with a `style` of font, fill, border and format |
| `image(string $cell, bytes $data, {width?, height?, offsetX?, offsetY?})` | a PNG, JPEG or GIF, read from its bytes; any other format throws `LogicError` |
| `chart(string $cell, Chart $chart, {width?, height?})` | a chart of the subset below |
| `print({...})` | page setup: `orientation`, `fitToPage`, `margins`, `header`, `footer`, `repeatRows`, `area` |
| `protect({password?, allow?})` | sheet protection |

Row heights, hidden rows and outline levels are `add`'s options, and hidden columns a column's
(`rule:core-classes/spreadsheet-styles-and-defaults`), because xlsx writes them with the row or before
it. For the same reason a `group` over rows is added before those rows, a `group` over columns and
`print`'s `fitToPage` before the sheet's first row, and each throws `LogicError` saying so when it
comes later. **A link goes to `http:`, `https:`, `mailto:` or a cell of the workbook**, and any
other target, such as `file:` or `javascript:`, throws `LogicError`, so a link built from user data
opens no local file and runs nothing. `Workbook::define` writes a defined name, and the workbook's
`properties` the document properties.
Rich text in one cell is `Cell::rich`. **Sheet protection locks the layout and is not security**: the
password is a hash any tool removes, which the reference card says.

**An image is its own zip entry.** One added before the sheet's first row is written by the call that
adds it. Once the rows have started, the sheet's own entry is open, and a zip entry cannot be split,
so the image's bytes wait until the sheet ends. A report with many images adds them before its rows,
or on a sheet of their own, to hold none of them.

**A table's top row is its header row**, and each column is named by its header cell, which needs a
name different from the others. Once rows are written only the sheet's header row is kept, so a
table added after its rows starts at that row, and one added earlier takes its names when the row is
written. **The totals row is the row after the table**: a table added after its last row writes it
at once, and one added earlier writes it right after its last row, so the program's next row comes
after it. A sheet that ends before a table's header row or totals row throws `LogicError`. A total
is `SUBTOTAL` over the column, which skips the rows a filter hides. `[@qty]` and `[qty]` are
qualified by the table's name in the totals row and in every row written after the table is added;
a row written before it keeps its formula text as given. The sheet's default autofilter is left out
where a table covers it, and a table overlaps no merged range and no other table.

**A conditional format's values are formula text**, so a text value is quoted. Its rules take
priority in the order they are added. A rule that formats cells needs a `style`, of which the font's
bold, italic, strike, underline and colour, the fill, the border and the format apply; a colour
scale, a data bar and an icon set take `colors` instead, and a `style` on one throws `LogicError`.
**The chart subset** is column, bar, line, area, pie, doughnut and scatter, and a
combination of two different types of column, line and area over one pair of axes, with a title,
axis titles, the legend, series colours and data labels. Any other combination throws `LogicError`.
A series names its ranges with `$sheet->column($key)`, so a chart over a stream covers every row the
sheet ends with: the chart is written when the sheet ends, and a range on its own sheet that ends at
the row `column` gives when the chart is added ends at the sheet's last row. Any other range is
written as given, and the chart holds no values. **A series' categories are a number reference**
when they are a column of the chart's own sheet whose every value below its header is a number or a date, which the
writer knows from one flag per column kept as rows pass, and a text reference otherwise. Its XML
follows `rust_xlsxwriter`'s output for the same chart, ported for the subset and attributed in the
module that writes it.

**An image or a chart is anchored from its cell to the cell its size ends at**, as Excel anchors
one, with that cell's offset. The end cell needs the width of every column and the height of every
row the anchor spans, and an anchor may be added after its rows have left. So the writer keeps the
open sheet's custom row heights, one entry per run of rows with the same height. A sheet with no
custom height keeps nothing, and an anchor added before its rows finds its end cell as they pass.

The tradeoffs. **Usability:** a report's merges, totals and chart are written after the rows they
summarise, in the order a program computes them. **Memory:** the parts are held until the sheet ends,
and they are small beside the rows.

- **Memory:** per writing request, each sheet-end part of the open sheet until the sheet is finished,
  and the bytes of each image added after the sheet's first row, until the sheet is finished.
- **Memory:** per writing request, the open sheet's custom row heights, 8 bytes per run of rows with
  the same height, and one flag per column, until the sheet is finished. A sheet with no custom height
  keeps none, and one whose every row differs from the row before keeps 8 MB at Excel's 1,048,576 rows.

**On disk.** Every sheet-end part above, called from `extensions/spreadsheet/nvs/Sheet.nvs` and
written by `extensions/spreadsheet/src/tail.rs`, `table.rs`, `conditional.rs`, `drawing.rs` and
`chart.rs`.

**Declined:**
- **Pivot tables, sparklines and threaded comments.** Each is a part with its own cache or model, for
  a feature a reader builds in Excel from the data in seconds.
- **Every chart type outside the subset, trendlines, error bars, secondary axes beyond the
  combination, chartsheets and 3D.** The subset is what reports use; the rest is Excel's to add.
- **An image that is not PNG, JPEG or GIF converted on the way in.** `Novis\Image` converts it in one
  call, and the writer stays a writer.
- **A chart's cached values, the `numCache` and `strCache` Excel writes beside each reference.** A
  chart added after its rows finds them gone, so a cache would mean keeping every cell of every sheet,
  and one only for a chart added first would make the file depend on call order. Excel and LibreOffice
  read the cells when they open the file, and every workbook is flagged to recalculate. A viewer that
  draws from the cache alone shows the chart empty. The Excel comparison removes the caches from
  Excel's file and names that in its harness.
