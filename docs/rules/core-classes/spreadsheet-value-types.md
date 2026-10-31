Every Novis value writes as one cell type and every cell reads as one Novis type, by two fixed tables,
so neither a write nor a read guesses from the value's content.

**The write table:**

| Novis value | Cell |
|---|---|
| `string` | a text cell, always, whatever it begins with (`rule:core-classes/spreadsheet-formula-is-a-value`) |
| `int`, `float` | a number; NaN and the infinities throw `LogicError` naming the column, for xlsx, ods and CSV alike |
| `decimal` | a number |
| `int` or `decimal` with more than 15 significant digits | a text cell, so no digit is lost |
| `bool` | `TRUE` or `FALSE` |
| `Date` | a serial with the format `yyyy-mm-dd` |
| `TimeOfDay` | a serial with the format `hh:mm:ss` |
| `Duration` | a serial with the format `[h]:mm:ss` |
| `DateTime` | its own clock time as a serial, with `yyyy-mm-dd hh:mm` |
| `Instant` | its clock time in the workbook's `zone`, with `yyyy-mm-dd hh:mm` |
| `null` | an empty cell |
| `Formula` | a formula, with its cached result when it carries one |
| `Cell::of($value, $style)` | the value's own cell, with that style on top of the row's and column's |

Any other value throws `LogicError` naming the column and the type. A column or `Style` `format`
replaces the table's format and never the cell type.

**The zone rule.** A serial has no zone, so an `Instant` needs one to become a clock time: the
workbook's `zone` option, and with none the write throws naming the column. A `DateTime` already
carries its clock time, which is written as it is. On read, `zone` (default `'UTC'`) is the zone a
read `DateTime` is placed in, and the clock time in the file is kept exactly; no conversion moves it.

**The read table:**

| Cell | Novis value |
|---|---|
| text, inline or shared | `string` |
| a whole number within ±2^53 | `int` |
| another number | `float` |
| `TRUE`, `FALSE` | `bool` |
| a number with a date format that has a date and a time | `DateTime` in `zone` |
| a number with a date format that has no time | `Date` |
| a number with a format that has only a time | `TimeOfDay` |
| a number with an elapsed-time format (`[h]`, `[mm]`, `[ss]`) | `Duration` |
| an error value | a `CellError` case |
| empty | `null` |
| a formula | its stored result, read by this table; `{formulas: true}` gives the `Formula` with that result as its `cached` |

A format is a date format when its code has a date or time token outside quotes and brackets, or it is
one of the built-in ids that are. Both epochs are read, 1900 with its leap-day bug and 1904.
`{text: [...]}` names columns that read as Excel's displayed text instead, and `{text: true}` reads
every column so (`rule:core-classes/spreadsheet-reads-as-a-stream`). A CSV cell is always `string`,
because CSV has no other type.

The tradeoffs. **Usability:** an import gets an `int` where the sheet has a whole number and a `Date`
where it has a date, with no parse in the program. **Simplicity:** one table per direction, and the
display text is one option away when the table is not what the program wants.

**On disk.** The write table, `Cell::of` and the zone rule: the Novis half picks a value's case
(`extensions/spreadsheet/nvs/Workbook.nvs`) and the component writes it
(`extensions/spreadsheet/src/cells.rs`). The read table and `{text: ...}`
(`extensions/spreadsheet/src/read/mod.rs`, `extensions/spreadsheet/src/format/mod.rs`).

**Declined:**
- **A `string` that looks like a number written as a number.** The cell would depend on the content,
  and a zip code or an account number would lose its leading zeros.
- **Writing an `Instant` in UTC when no `zone` is set.** Every reader in another zone would see the
  wrong clock time with nothing to tell them; a throw names the missing option once.
- **Reading every number as `float`.** An id column would come back as `1042.0`, and ±2^53 is exactly
  the range in which `float` holds a whole number without loss.
- **A read `DateTime` converted into the program's zone.** The file holds a clock time and no zone, so
  any conversion would invent the zone it converts from.
