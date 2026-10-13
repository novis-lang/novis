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
is never resolved, and an image placed on write is bytes the caller supplies. CSV goes through the
same classes and is parsed and formatted by `Core\Csv` in the Novis half, so CSV has one parser and
one API for the job.

**Reading evaluates nothing.** A cell reads as its stored scalar, or as its formula text with the cached
result the file's author wrote beside it, and the two are distinguishable at the API, because a cached
value is a claim by the author rather than a computation by us
(`rule:core-classes/spreadsheet-value-types`). VBA and embedded OLE objects are inert payload the reader
may list but never executes. The XML parser resolves no entity and loads no DTD. Cells read from
`tainted` bytes are `tainted`.

The component's tests owe, asserted inside the guest: a read and a write with no capability grant that
succeed with no network or filesystem access observable; and an xlsx whose sheet XML declares an
external entity, and one carrying an external-workbook reference, each read with the reference
surfaced as data and nothing resolved.

- **Memory:** nothing until a program first uses `Novis\Spreadsheet`. Then the compiled module, held
  once per process and shared by every core, and per calling request one instance under the request's
  cap, freed when the request ends. What a write and a read hold inside it are their own fragments'.

**Not shipped.** Nothing of `Novis\Spreadsheet` is on disk.

**Declined:**
- **A first-party `[[extension]]` package, `nvs/spreadsheet`.** It is one configuration step for a
  feature every web application uses, and `Novis\` is reserved for what the binary carries.
- **A preopen for the component, so it writes the file itself.** That is a grant of a directory to a
  parser of hostile bytes, and the Novis half already holds the file the program opened.
- **CSV left to `Core\Csv` alone, beside the component.** An import that accepts xlsx and CSV would be
  two APIs with two row shapes; through one `Reader` it is one loop.
- **Written in Novis.** It would reimplement OOXML and BIFF parsing that production crates already
  carry, unfuzzed and ours to maintain, and buy nothing the wasm build does not give. Novis code
  belongs in the builder.
- **Exposing the crates' surfaces directly.** That is several disjoint APIs for one job inside a single
  component.
