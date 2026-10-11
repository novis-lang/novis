The writer never emits JavaScript, launch actions, an open action, form fields, or any construct that
makes a reader fetch or execute on open. Links exist only where the source wrote one with an `http(s):`,
`mailto:` or `#anchor` target. Rendering `tainted` input is safe by construction — the render is a
transform, not a sink — and a program embedding untrusted markup that wants its *tags* constrained
sanitizes first (`rule:core-classes/html-sanitize`).

The one embedded file the writer ever emits is a **Factur-X invoice XML**, under PDF/A-3b only
(`rule:core-classes/pdf-factur-x-is-the-one-attachment`). An embedded file a reader opens is a
delivery channel for anything, so no other attachment exists, and the invoice XML is data the reader
parses, never opens.

The writer embeds **no timestamp unless the program passes one, and no generator entropy**: the
document id is derived from the content, and nothing depends on hash-map order or on float formatting
that varies between runs. So two renders of equal input are byte-equal, including encrypted output,
whose salts and key are derived the same way (`rule:core-classes/pdf-standards-and-passwords`). That
turns a document test into a byte comparison and removes the class of flaky fixture every PDF test
suite otherwise grows.

The component's fixtures owe a scanner over the output of the whole fixture corpus that finds no
`/JavaScript`, `/Launch` or `/OpenAction` object and no `/EmbeddedFile` outside the Factur-X fixtures,
and the same input rendered twice and compared byte for byte, encrypted output among it.

**Not shipped.** There is no PDF component in the tree.

**Declined:**
- **Any attachment besides the invoice XML.** A program that needs to send several files sends them
  beside the PDF.
- **A creation time read from a clock.** The guest has none, and a time the program did not ask for
  breaks byte equality.
