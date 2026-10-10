The writer emits no macros, no embedded executables and no launch-shaped content; hyperlinks exist
only where the document wrote them.

It embeds **no timestamps and no generator entropy** — document properties carry fixed dates unless
the caller sets them, and archive entries a fixed time — so equal input gives byte-equal output and a
document test is a byte comparison. This is the same rule the PDF writer holds
(`rule:core-classes/pdf-output-is-inert`), and it exists for the same reason: a test that cannot
compare bytes ends up comparing nothing.

The component's fixtures owe a filled `.xlsm` template whose output a scanner finds no `vbaProject`
stream in, and the same input serialized twice and compared byte for byte.

**Not shipped.** There is no spreadsheet package in the tree.
