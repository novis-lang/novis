`Reader::open($source)` reads a `Core\IO\File` the program opened, or `bytes` it holds, and xlsx and
xlsb are read as a stream of byte ranges, so a 100 MB upload never enters the guest whole; xlsm, xls
and ods are read whole, and CSV goes to `Core\Csv`.

**How the guest reads a range.** Every reader call lends the source as a `mixed` value, and the guest
reads ranges of it through a host import, an accessor of the `value` handle that returns the bytes at
an offset and length of a `bytes` value or of an open `Core\IO\File`. The guest keeps a read-ahead
buffer, reads the zip directory from the end, and inflates each part as it parses it. The import
grants nothing: it reads only what the program lent to this call, and the handle stops being valid
when the call returns (`rule:packaging/values-cross-as-handles`), so the source is lent again on every
call. This is chosen against a pull loop in the Novis half, in which the guest would return "bytes
needed at offset N, length L" and the Novis code would read them and call again, for two reasons. On
request latency, the import costs one host call per read-ahead block and runs no Novis code between
them, where the pull loop costs a guest return, a Novis read and a guest call for every block. And
the parsers read synchronously from a `Read + Seek`: calamine does, for xlsb and the whole formats,
and a pull loop would need every parser rewritten as a resumable state machine.

**Formats** are told apart by their bytes: a zip is xlsx, xlsm or ods by its content types, an OLE
compound file is xls, a zip with `xl/workbook.bin` is xlsb, and anything else is CSV. xls holds at
most 65,536 rows. An xlsm's macros are listed by `macros()` and never run.

**`rows($sheet, {...})`** takes the sheet by name or by position from 0 and yields one `Row` per row:
its `number` is the **row number in the file**, and its `values` the row's array. The row number is
not the `foreach` key, because a cursor carries no key (`rule:iteration/foreach-subjects`), and a
generator yields a class and not a shape. The generator only yields: a batch's shapes are read in an
ordinary method, which the compiler lowers where it does not yet lower them inside a generator.

| Option | Default | What it does |
|---|---|---|
| `header` | `false` | `true` reads the first row with a value at or after `from` as the header; an `int` names the header's row number. The values are then keyed by header text |
| `from` | the first row | the first row number to read |
| `limit` | none | the most rows to yield |
| `columns` | every column | `['A:C', 'F']` |
| `zone` | `'UTC'` | the zone a read `DateTime` is in |
| `text` | none | the columns, by header or letter, that read as their displayed text; `true` for all |
| `formulas` | `false` | `true` reads a formula cell as its `Formula` |
| `emptyRows` | `'skip'` | `'keep'` yields an empty row |
| `merged` | `'first'` | `'fill'` gives every cell of a merged range its value |
| `hidden` | `'read'` | `'skip'` leaves hidden rows out |
| `locale` | `'en'` | the separators `text` renders with |

With a header, **every row has every header key**, and a missing cell is `null`; a duplicate header
gets ` (2)`, ` (3)` and on, and an empty header cell is its column letter. Without one, a row is keyed
by column letter. A CSV read takes the same options and gives the same row shape, and its cells are
all `string`.

**`sheets()`** gives one shape per sheet: `name`, `position`, `visibility` (`Reader::Visibility`), `rows`
and `columns` as the file declares them, and `kind` (`'worksheet'` or `'chartsheet'`). **The extras
read on request**, each for one sheet or the workbook: `merged`, `links`, `notes`, `names`, `tables`,
`images` (as bytes with their anchor), `properties` and `macros`. An external link, a linked image or a
data connection reads as data and is never followed (`rule:core-classes/spreadsheet-has-no-io`).

**Shared strings are kept in one arena** with `u32` offsets, never one allocation per entry, and the
reader reads `styles.xml`'s number formats and each cell's style index, which the read table's date
rule and the `text` option need. **A part streamed and never held** — a sheet read row by row — is
bounded by the decompression ratio alone; a part held whole, such as the shared strings, keeps both
bounds (`rule:core-classes/decompression-bound`).

The tradeoffs. **Performance:** one host call per read-ahead block, one guest call per batch of rows,
and no Novis code between blocks. **Usability:** typed values, the header's keys on every row, and row
numbers to report errors by. **Simplicity:** one `Reader` for every format, CSV included. **Memory:**
spent on the shared strings, which a file may make large, and bounded by the request's cap.

- **Memory:** per reading request, the read-ahead buffer, the inflater's window, one batch of rows,
  the number formats, and the shared strings of the file being read in one arena; for xlsm, xls and
  ods, the whole file. All of it is inside the request's cap and freed when the request ends.

**The Novis half** asks for 1,000 rows a call, with `from` one past the last row it was given.
`header: true` costs one call more, for the number of the first row with a value. Text read from a
file is `tainted`, so a `Formula` read with `formulas: true` carries a number result and not a text
one. The engine's resource is `reading`, because a manifest's resource is a class in
`Novis\Spreadsheet` and `Reader` is the class over it.

**Not shipped.** The extras, `merged: 'fill'`, `text`, and the xlsb, xlsm, xls and ods formats. The
`value` accessor reads a range of `bytes` and not yet of an open `Core\IO\File`, and each call copies
a `bytes` source whole as it crosses. `Reader::open`, `sheets()` and `rows()` read an xlsx.

**Declined:**
- **A pull loop in the Novis half.** It costs a guest return and a Novis call per block on the request
  path, and rewrites every parser as a resumable machine (the paragraph above).
- **The whole file copied into the guest.** A 100 MB upload would cost 100 MB of guest memory beside
  the program's own copy.
- **A preopen for the uploaded file.** It is a grant to a parser of hostile bytes, and a range import
  on a lent value grants nothing (`rule:packaging/a-guest-has-no-ambient-authority`).
- **Whole-sheet reads into one block.** A large sheet would cost its size; a batch of rows costs the
  same for every size.
- **A per-cell accessor.** `rule:core-classes/spreadsheet-bulk-boundary` declines it for good.
