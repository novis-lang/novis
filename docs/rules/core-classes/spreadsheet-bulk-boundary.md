The builder assembles the whole document — sheets, cells, styles, charts — as Novis values, and
**one call serializes it**. Reading opens a workbook once and pulls whole sheets or declared ranges
as bulk blocks. **There is no per-cell accessor, and none is added later**: a member that would be
called in a loop over the file's own contents costs the boundary, not the work — the same refusal the
image component makes.

Before any sheet buffer is allocated, the component reads the declared dimensions and the container's
declared uncompressed sizes, and refuses a workbook over a cell cap with a throw naming the cap and
the declared size. The cap is policy and the sandbox's memory cap is the backstop, which is the shape
the image component's pixel cap already has.

What it spends, per request that calls it: the document model and its serialized form, inside the
extension's memory cap, none of it outliving the request. A million-row export belongs in the queue,
and the write path must offer a bounded-memory streaming mode, so a large export costs rows in
flight rather than rows total.

**Not shipped.** There is no spreadsheet package in the tree.
