Spreadsheet reading and generation are one first-party Tier 1 extension package, `nvs/spreadsheet`,
sandboxed, with `Novis\Spreadsheet\Engine` as its one registered class and everything else under
`Novis\Spreadsheet` Novis source that calls it. It is not `Core`: a PhpSpreadsheet-sized API is a
permanent simplicity cost, and its parsers unsandboxed in every in-flight request is exactly the
outcome the tier system exists to prevent. It is not Native either, because nothing in it holds
connection state or privilege, and the sandbox is the feature. CSV stays `Core\Csv`'s, so each job
has one way to write it.

The placement holds because an uploaded workbook is the tier's exact hostile-bytes material: xlsx is a
zip of XML, where external entities and decompression bombs are the genre's vulnerability history, and
legacy xls is an OLE compound binary. Parsing or serializing a document costs milliseconds, so the
boundary crossing is noise.

A workbook crosses the boundary as bytes and the result returns as bytes; **the component fetches
nothing**. An external-workbook reference, a linked image or a remote data connection in a read file
comes back as **data** — the reference itself — and is never resolved. An image placed on write is
bytes the caller supplies. A workbook is self-contained, so this needs no asset map.

**Reading evaluates nothing.** A cell reads as its stored scalar, or as formula text plus the cached
result the file's author wrote beside it, and the two are distinguishable at the API, because a
cached value is a claim by the author rather than a computation by us. VBA and embedded OLE objects
are inert payload the reader may enumerate but can never execute. Cells read from `tainted` bytes are
`tainted`.

The component's fixtures owe, asserted inside the guest: a read and a write with no capability grants
that succeed with no network or filesystem access observable; and an xlsx whose sheet XML declares an
external entity, and one carrying an external-workbook reference, each read with the reference
surfaced as data and nothing resolved.

**Not shipped.** There is no spreadsheet package in the tree; M17 builds it.

**Declined:**
- **Written in Novis.** It would reimplement OOXML and BIFF parsing that production crates already
  carry, unfuzzed and ours to maintain, and buy nothing the wasm build does not give. Novis code
  belongs in the builder.
- **Leaving the category to the third-party channel.** The read surface belongs where the sandbox and
  the cap are always present, the typed formula only protects when it is the one way everybody writes
  a cell (`rule:core-classes/spreadsheet-formula-is-a-value`), and the existing engines make
  first-party assembly cheap.
- **Exposing the crates' surfaces directly.** That is three disjoint APIs for one job inside a single
  component.
