`Novis\Spreadsheet` is the fourth component built into every `nvs` binary
(`rule:packaging/the-first-party-components-are-built-in`), sandboxed, always present under
`Novis\`, with `Novis\Spreadsheet\Engine` as its manifest class and every other class Novis source
that calls it (`rule:core-classes/spreadsheet-roster`). It is not `Core`: a spreadsheet API is a
permanent simplicity cost, and its parsers unsandboxed in every in-flight request is the outcome the
tier system exists to prevent. It is not Native either, because nothing in it holds connection state
or privilege, and the sandbox is the feature. It is not an `[[extension]]` package, because an
export and an import of an upload are jobs nearly every web application has, and a program could not
know whether the package exists without asking its deployment.

The placement holds because an uploaded workbook is the tier's exact hostile-bytes material: xlsx is a
zip of XML, where external entities and decompression bombs are the genre's vulnerability history, and
legacy xls is an OLE compound binary. One guest call per batch of rows or per read-ahead block makes
the boundary crossing noise beside the work (`rule:core-classes/spreadsheet-bulk-boundary`).

**The component never touches disk, and is granted nothing.** Writing returns compressed chunks, and
the Novis half sends them on to the response, the file or the memory the program chose
(`rule:core-classes/spreadsheet-writes-as-a-stream`). Reading pulls byte ranges of a `Core\IO\File`
the program opened, or of `bytes` it holds, through the call that lent them
(`rule:core-classes/spreadsheet-reads-as-a-stream`). No preopen and no outbound HTTP is granted to the
component, under any configuration. **The component fetches nothing**: an external-workbook reference,
a linked image or a data connection in a read file comes back as **data** — the reference itself — and
is never resolved, and an image placed on write is bytes the caller supplies.

**CSV goes through the same classes and is parsed and formatted by `Core\Csv` in the Novis half**, so
CSV has one parser and one API for the job. `Reader::open` reads a source as CSV when it does not
start with a zip's `PK` and a control byte and its start is UTF-8 (`bytes` decoded whole, a file's
first four bytes); anything else goes to the engine, which throws for a file that is no workbook. A
CSV is one sheet, `Sheet1`, with no extras. `rows` takes every option and gives the xlsx row shape:
the header keys numbered and lettered as the engine does, an empty field an empty cell, a short
record's missing keys `null`, a record numbered by its place in the file. Every value is a `string`.
`Workbook` writes CSV for a name ending in `.csv` or `{format: 'csv'}`: each batch of rows goes
through `Core\Csv::format` with each value as its text, and a second sheet throws naming both. The
engine still opens a writer, which checks the sheet name and logs a workbook never finished, and its
bytes are dropped, so styles, widths and every sheet-end part are left out. **A string that starts
with `=`, `+`, `-`, `@`, a tab or a carriage return is written with a `'` first**, because Excel reads
such a field as a formula, and a string is always text; reading does not strip it.

- **Performance:** a CSV write still instantiates the component once, for the writer's checks; the
  rows never cross. A read of `bytes` parses the whole text each time `sheets` or `rows` walks it.
- **Memory:** a read of `bytes` holds the decoded text and its records, per request; a read of a
  `Core\IO\File` holds one record (`Core\Csv::rows`). A write holds one batch of rows.
- **Usability:** one import loop for xlsx and CSV. The `'` a guarded string carries is in the file
  for every tool that reads it, not only for Excel.

**Reading evaluates nothing.** A cell reads as its stored scalar, or as its formula text with the cached
result the file's author wrote beside it, and the two are distinguishable at the API, because a cached
value is a claim by the author rather than a computation by us
(`rule:core-classes/spreadsheet-value-types`). VBA and embedded OLE objects are inert payload the reader
may list but never executes. The XML parser resolves no entity and loads no DTD.

**Everything read is `tainted`.** `Reader::open` takes `tainted bytes`, so an upload reads, and the
source is held `tainted` whatever the program passed, because a workbook is outside data even when
the program holds it as plain `bytes`. Every string and `bytes` value `Reader` returns is `tainted`:
cells, header keys, sheet names, macro names, merged ranges and every extra. A `Formula` read with
`formulas: true` carries its text as a `tainted string`, so `Formula::text` is `tainted` for every
formula. The writer launders it with `Core\Taint::assertTrusted` at the one place a `Formula` becomes
a formula, because passing a `Formula` is the program's own choice to write one, and a string is
always text. Cost: a program that reads `->text` of a formula it made must assert it trusted too.
The engine's own results are plain shapes, because an extension call's contagion does not reach a
shape's fields (`crates/nvs-types/src/expr/quals.rs`'s `tainted_result`), so the `tainted` types are
written in `Reader`'s declarations, and a sheet name goes back to the engine through one
`assertTrusted` naming why.

The component's tests owe, asserted inside the guest: a read and a write with no capability grant that
succeed with no network or filesystem access observable; and an xlsx whose sheet XML declares an
external entity, and one carrying an external-workbook reference, each read with the reference
surfaced as data and nothing resolved.

- **Memory:** nothing until a program first uses `Novis\Spreadsheet`. Then the compiled module, held
  once per process and shared by every core, and per calling request one instance under the request's
  cap, freed when the request ends. What a write and a read hold inside it are their own fragments'.

**Not shipped.** A CSV dialect other than `,` and `"`, and a CSV in another encoding than UTF-8
(gap:nvs-spreadsheet/spreadsheet-csv-dialects).

**Declined:**
- **A first-party `[[extension]]` package, `nvs/spreadsheet`.** It is one configuration step for a
  feature every web application uses, and `Novis\` is reserved for what the binary carries.
- **A preopen for the component, so it writes the file itself.** That is a grant of a directory to a
  parser of hostile bytes, and the Novis half already holds the file the program opened.
- **CSV left to `Core\Csv` alone, beside the component.** An import that accepts xlsx and CSV would be
  two APIs with two row shapes; through one `Reader` it is one loop.
- **A CSV string written as it is.** A string such as `=HYPERLINK(...)` from a user would run as a
  formula in Excel, and the program wrote a string, which is always text.
- **Typed CSV values, a number read as `int`.** CSV has no types, so a guess would read `00123` as
  `123`; a program converts the `string` it expects with `as ?int`.
- **Written in Novis.** It would reimplement OOXML and BIFF parsing that production crates already
  carry, unfuzzed and ours to maintain, and buy nothing the wasm build does not give. Novis code
  belongs in the builder.
- **A plain `Formula::text`, with the reader asserting a read formula trusted.** That launders a
  file's text into every sink, an HTML page included, to save a program one `assertTrusted` on a
  formula it made itself.
- **Exposing the crates' surfaces directly.** That is several disjoint APIs for one job inside a single
  component.
