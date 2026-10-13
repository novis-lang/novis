The writer emits no macros, no embedded executables and no launch-shaped content; hyperlinks exist
only where the program wrote them.

It embeds **no timestamps and no generator entropy** — document properties carry fixed dates unless
the program sets them, archive entries a fixed time, and no output depends on a map's order — so equal
input gives byte-equal output, for a write and for a template fill alike, and a document test is a
byte comparison. This is the same rule the PDF writer holds (`rule:core-classes/pdf-output-is-inert`),
and it exists for the same reason: a test that cannot compare bytes ends up comparing nothing.

The component's tests owe a filled `.xlsm` template whose output a scanner finds no `vbaProject`
part in, and the same input written twice and filled twice, each pair compared byte for byte.

**Not shipped.** Nothing of `Novis\Spreadsheet` is on disk.
