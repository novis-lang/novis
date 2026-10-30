A style is a `Style::of({...})` value, layered column, then row, then cell, and a sheet with a header
row is styled by default: a bold header with a bottom border, frozen, filtered, and every column
`width: 'auto'`.

**The style fields**, every one optional:

| Field | Values |
|---|---|
| `font` | a font name, such as `'Calibri'` |
| `size` | points, a `float` |
| `bold`, `italic`, `strike` | `bool` |
| `underline` | `true`, `'single'` or `'double'` |
| `color`, `fill` | `'#RRGGBB'` |
| `border` | `'thin'`, `'medium'`, `'thick'`, `'dashed'`, `'dotted'` or `'double'` on every side, or `{top?, right?, bottom?, left?}` of those |
| `borderColor` | `'#RRGGBB'` |
| `align` | `'left'`, `'center'`, `'right'`, `'fill'` or `'justify'` |
| `valign` | `'top'`, `'center'` or `'bottom'` |
| `wrap` | `bool` |
| `rotate` | degrees, -90 to 90 |
| `indent` | an `int` |
| `format` | a format code, or a shortcut below |
| `locked`, `hideFormula` | `bool`, which take effect only on a protected sheet |

**The format shortcuts** are `'date'` (`yyyy-mm-dd`), `'datetime'` (`yyyy-mm-dd hh:mm`), `'time'`
(`hh:mm:ss`), `'duration'` (`[h]:mm:ss`), `'integer'` (`0`), `'decimal'` (`0.00`), `'money'`
(`#,##0.00`), `'percent'` (`0%`) and `'text'` (`@`). Any other string is a format code, written as it
is.

**Layering.** A cell's style is each field from the most specific layer that sets it: the cell's own
(`Cell::of`), then the row's (`add`'s `style`), then the column's. A field no layer sets is Excel's
default. Every distinct combination becomes one entry of `styles.xml` at `finish()`, so equal
combinations share one entry, and Excel's limit of 65,490 cell formats throws naming it.

**Columns** are a sheet option, `columns: {key: {...}}`, keyed by the row's array keys, in the order the
header is written. A column takes `header` (its header text, default the key), `width` (characters,
or `'auto'`), `format`, `style`, `hidden` and `formula`
(`rule:core-classes/spreadsheet-formula-is-a-value`). A row key no column names is written after the
named ones in the order of the first row; a column a row lacks is an empty cell. Rows that are lists
of plain lists have no keys, so they have no header and no defaults.

**The defaults, whenever a sheet has a header row**, in `fromRows` and `sheet()` alike: the header row
bold with a thin bottom border (`headerStyle`), frozen below it (`freeze`), with an autofilter over the
header (`autofilter`), and every column `width: 'auto'`. Each turns off on its own:
`{freeze: false, autofilter: false, headerStyle: false}`, or a column's own `width`. `headerStyle` is
`false` and not `null`, the way `freeze` and `autofilter` are, because an options field set to `null`
reads the same as one left out. `headerStyle: Style::of({...})` replaces the default style. `header:
false` writes no header row, and so none of these.

**Auto width** measures the header and the rows the Novis half holds before the first chunk leaves —
the first 1,000 — or every row when `fromRows` is given an array. A cell's width is its text as Excel
displays it, through the number-format renderer the reader's `text` option uses, counting a wide East
Asian character as two, plus one, capped at 60. There are no font metrics.

The tradeoffs. **Usability:** an export looks finished with no option at all, and a style is a value
a program names once and reuses. **Performance:** the 1,000-row window keeps a download's first byte
from waiting on the whole query. **Memory:** the window is up to 1,000 rows held once, at the start of
a sheet.

- **Memory:** per writing request, one entry per distinct style combination until `finish()`, and the
  auto-width window of at most 1,000 rows before the first chunk leaves.

**On disk.** `Style::of`, `Cell::of`, the row's `style`, `columns` with `header`, `width`, `format`,
`style`, `hidden` and `formula`, the header row and its four defaults, and auto width through the
number-format renderer (`extensions/spreadsheet/nvs/Workbook.nvs`, `Style.nvs`,
`extensions/spreadsheet/src/width.rs`).

**Not shipped.** The style enums under `Style`: they are at namespace scope
(gap:nvs-spreadsheet/spreadsheet-style-enums).

**Declined:**
- **A style as an options bag on every call, with no value.** A report repeats the same few styles on
  thousands of rows, and a value compares and dedupes once.
- **Unstyled by default.** The common export is a table with a header, and the four defaults are
  what every user would otherwise add by hand.
- **Auto width over every row of a stream.** It would hold the sheet until the end, which is the memory
  streaming exists to avoid.
- **Font metrics for auto width.** It needs every font's tables in the guest for a width a reader
  changes with one drag; characters as displayed are close enough, and the cap keeps one long cell
  from taking the screen.
