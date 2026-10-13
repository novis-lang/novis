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
it. `Workbook::define` writes a defined name, and the workbook's `properties` the document properties.
Rich text in one cell is `Cell::rich`. **Sheet protection locks the layout and is not security**: the
password is a hash any tool removes, which the reference card says.

**An image is its own zip entry**, written as soon as it is added, so a report with many images holds
none of them. **The chart subset** is column, bar, line, area, pie, doughnut and scatter, and a
combination of two of them, with a title, axis titles, the legend, series colours and data labels. A
series names its ranges with `$sheet->column($key)`, so a chart over a stream covers every row the
sheet ends with. Its XML follows `rust_xlsxwriter`'s output for the same chart, ported for the subset
and attributed in the module that writes it.

The tradeoffs. **Usability:** a report's merges, totals and chart are written after the rows they
summarise, in the order a program computes them. **Memory:** the parts are held until the sheet ends,
and they are small beside the rows.

- **Memory:** per writing request, each sheet-end part of the open sheet until the sheet is finished,
  and nothing of an image after the call that adds it.

**Not shipped.** Nothing of `Novis\Spreadsheet` is on disk.

**Declined:**
- **Pivot tables, sparklines and threaded comments.** Each is a part with its own cache or model, for
  a feature a reader builds in Excel from the data in seconds.
- **Every chart type outside the subset, trendlines, error bars, secondary axes beyond the
  combination, chartsheets and 3D.** The subset is what reports use; the rest is Excel's to add.
- **An image that is not PNG, JPEG or GIF converted on the way in.** `Novis\Image` converts it in one
  call, and the writer stays a writer.
