Spreadsheet reading and generation are one first-party Tier 1 extension package, sandboxed, with one
registered class and everything else Novis source that calls it. It is not `Core`: a
PhpSpreadsheet-sized API is a permanent simplicity cost, and its parsers unsandboxed in every
in-flight request is exactly the outcome the tier system exists to prevent.

A workbook crosses the boundary as bytes and the result returns as bytes; **the component fetches
nothing**. An external-workbook reference, a linked image or a remote data connection in a read file
comes back as **data** — the reference itself — and is never resolved. An image placed on write is
bytes the caller supplies. A workbook is self-contained, so this needs no asset map.

**Reading evaluates nothing.** A cell reads as its stored scalar, or as formula text plus the cached
result the file's author wrote beside it, and the two are distinguishable at the API, because a
cached value is a claim by the author rather than a computation by us. VBA and embedded OLE objects
are inert payload the reader may enumerate but can never execute. Cells read from `tainted` bytes are
`tainted`.

**Not shipped.** There is no spreadsheet package in the tree; M17 builds it.
