`Novis\Spreadsheet` is twelve classes and one enum, with `Engine` the manifest class whose static
methods are `wit/spreadsheet.wit`'s exports and which no program calls directly:

| Class | What it is | Its members |
|---|---|---|
| `Workbook` | A workbook being written to one target | `createForDownload(string $name, {…})`, `createForFile(string $path, {…})`, `createForStream(Core\IO\File $file, {…})`, `createInMemory({…})`, `fromRows(iterable $rows, {…}): Export`; `sheet(string $name, {…}): Sheet`, `define(string $name, string $reference): Workbook`, `finish(): Written` |
| `Sheet` | One sheet of a `Workbook`, written row by row | `add(array $row, {…}): Sheet`, `addAll(iterable $rows): Sheet`, `cell(string\|int $column, int $row): string`, `column(string\|int $column): string`, and the sheet-end parts `merge`, `link`, `note`, `validate`, `group`, `table`, `conditional`, `image`, `chart`, `print`, `protect` |
| `Written` | What `finish()` returns | `rows: array<int>` by sheet name, `size: int`, `bytes: ?bytes` |
| `Export` | A whole workbook from one call, written where its last call says | `download(string $name): void`, `save(string $path): Written`, `to(Core\IO\File $file): Written`, `bytes(): bytes` |
| `Reader` | A workbook or CSV being read | `open(Core\IO\File\|bytes $source): Reader`, `sheets(): array<{…}>`, `rows(string\|int $sheet, {…}): Iterator<Row>`, and the extras `merged`, `links`, `notes`, `names`, `tables`, `images`, `properties`, `macros` |
| `Row` | One row `rows` read | `number: uint`, the row number in the file, and `values: array` by header text or column letter |
| `Template` | A designed workbook to fill | `open(Core\IO\File\|bytes $source): Template`, `set(string $address, mixed $value): Template`, `fill(array $data): Export` |
| `Style` | A set of style fields | `of({…}): Style` |
| `Cell` | One value with its own style, or rich text | `of(mixed $value, Style $style): Cell`, `rich(array<{text: string, style?: Style}> $runs): Cell` |
| `Formula` | A formula cell's text and its cached result | `of(string $text, {cached?: mixed}): Formula`, `text: string`, `cached: mixed` |
| `Chart` | One chart of the subset | `column`, `bar`, `line`, `area`, `pie`, `doughnut`, `scatter` (each `({…}): Chart`), `series(string $values, {…}): Chart`, `combine(Chart $other): Chart` |
| `Engine` | The manifest class | one static method per export |

The enum is `CellError`, the cell error values `#NULL!`, `#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`,
`#NUM!`, `#N/A`, `#GETTING_DATA`, `#SPILL!` and `#CALC!` as the cases `Null`, `DivZero`, `Value`,
`Ref`, `Name`, `Num`, `NotAvailable`, `GettingData`, `Spill` and `Calc`. It stays at namespace scope,
because a cell value is `mixed` in every signature that carries one and no member names it
(`rule:enums/an-enum-lives-in-the-class-that-uses-it`). `Reader::Visibility` (`Visible`, `Hidden`,
`VeryHidden`) is the one owned enum, the type of a `sheets()` entry's `visibility`.

**The target comes first, and the verb last.** A program that writes row by row names where the bytes
go when it creates the `Workbook`, so a download's headers leave at once; a one-call helper
(`Workbook::fromRows`, `Template::fill`) names it last, with the verb (`rule:core-api/one-paradigm-per-operation`
holds each job to one of the two). **The last call of a write is `finish()`**, or the `Export`'s verb.
Each options shape is the home fragment's: the workbook's and the sheet's in
`rule:core-classes/spreadsheet-writes-as-a-stream`, the columns and styles in
`rule:core-classes/spreadsheet-styles-and-defaults`, the reader's in
`rule:core-classes/spreadsheet-reads-as-a-stream`, the template's in
`rule:core-classes/spreadsheet-template-fill`, and the formula helpers in
`rule:core-classes/spreadsheet-formula-is-a-value`. Every options bag is a shape
(`rule:core-api/options-bag`).

**An error throws the world's three classes.** A call the program got wrong — a bad sheet name, an
`Instant` with no zone, a row past the grid, a placeholder the data lacks — throws `LogicError`
naming the rule it broke and the cell, column or sheet. A file that does not read — a broken archive,
a lying record length, a bomb — throws `ParseError`. Anything else the component reports throws
`RuntimeError`. A trap throws `ExtensionError`, and the request's memory or CPU cap ends the request
(`rule:packaging/a-guest-crash-throws`).

**On disk.** Every member the table names but `Cell::rich`, in `extensions/spreadsheet/nvs/`, and
the enums in `extensions/spreadsheet/manifest.json`. A `Style` also has the readonly `fields` and
`key` that `Workbook` reads.

**Not shipped.** `Cell::rich`: the writer writes rich text, and no Novis method reaches it, so a
`Cell` is `of` alone (gap:nvs-spreadsheet/spreadsheet-cell-rich). `Visibility` is at namespace
scope with the style enums, not owned by `Reader` (gap:nvs-spreadsheet/spreadsheet-style-enums).

**Declined:**
- **One `Spreadsheet` class with every member.** Writing, reading and filling share no state, and a
  class per job keeps each card short.
- **`createForMemory`.** `createInMemory` says where the bytes stay, the way the other three name a
  target.
- **A `CellError` owned by `Reader`.** No signature names it, so the enum rule places it in the
  namespace.
