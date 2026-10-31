A workbook crosses the boundary a batch of rows at a time on write and a read-ahead block and a batch
of rows at a time on read, **never a cell at a time**, and Excel's own grid and limits are the only
limits the component adds. **There is no per-cell accessor, and none is added later**: a member that
would be called in a loop over the file's own contents costs the boundary, not the work — the same
refusal the image component makes (`rule:core-classes/image-pipeline`).

**Excel's limits throw, naming the limit and the place**: 1,048,576 rows and 16,384 columns, a row or
column index past them in a read file included; 32,767 characters in a cell; 65,490 cell formats;
8,192 characters in a formula; 255 characters in a defined name and in a chart or axis title.
**A sheet name** has 1 to 31 characters, none of `[ ] : * ? / \`, no control character, no `'`
first or last, and is not `History` in any case. One function checks it (`check_sheet_name` in
`extensions/spreadsheet/src/writer.rs`), for `Workbook::sheet`, a defined name's reference and a
chart's series range. **A defined name** in A1 or R1C1 form (`C3`, `R1C1`, `R`, `RC2`) throws. A merged range past the grid throws, and so does a record whose declared
length is past the bytes left. Every other bound is the request's: its memory and CPU caps hold the
guest (`rule:packaging/a-guest-runs-under-the-requests-budget`), and the decompression bound holds
every inflated part (`rule:core-classes/decompression-bound`). **There is no cell cap.** A file's
declared dimensions and declared sizes are its author's numbers, so no member trusts them, not even as
a hint: memory is measured as it is spent.

- **Memory:** a batch of rows on each side of the boundary, never the sheet. Each crossing is charged
  to the request while it is converted (`crates/nvs-ext/src/call.rs`'s module doc).

**On disk.** Every limit above throws: the writer's in `extensions/spreadsheet/src/`, and the
reader's under `extensions/spreadsheet/src/read/`.

**Declined:**
- **A cell cap with its own knob, read from the declared dimensions.** A streamed read holds a batch
  whatever the sheet's size, so a cap would refuse large files that cost nothing, and the dimensions
  it would read are the attacker's to write. The request's memory cap is the bound that measures.
- **A per-cell accessor.** Called in a loop over a sheet, its cost is the boundary crossed once per
  cell.
- **Whole sheets as one block.** It holds the sheet, which a batch does not.
