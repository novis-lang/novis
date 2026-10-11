**Every render is a tagged PDF**: headings, paragraphs, lists, tables with their header cells, figures
with their `alt` text and the document's language come from the HTML, with no option to turn it off.
`Document::standard` adds one of **PDF/A-2b, PDF/A-3b or PDF/UA-1**, and a document that breaks the
standard it chose throws `LogicError` naming what broke — an image with no `alt` under PDF/UA-1, a
font that cannot be embedded under PDF/A — through `krilla`'s own validation. One standard per
document, because `krilla` validates against one.

**`Document::password`** encrypts with **AES-256, revision 6**, and nothing weaker. Its salts and its
file key are derived from a hash of the document and the password, so equal input stays byte-equal
(`rule:core-classes/pdf-output-is-inert`). A password beside a PDF/A standard throws `LogicError`,
because PDF/A forbids encryption.

Tagging costs a structure tree the size of the element tree and buys a document a screen reader can
read; it is the default because a program that never thought about accessibility should still produce
an accessible file. AES-256 alone is offered because every reader made in the last fifteen years opens
it and the weaker revisions are broken.

- **Memory:** the structure tree, beside the layout tree inside the request's cap.

**Not shipped.** There is no PDF component in the tree.

**Declined:**
- **RC4 and AES-128 encryption, and owner permissions.** RC4 is broken, and permission flags are a
  request to the reader that any tool ignores.
- **Form filling and signatures.** Each is a second model of the file: fields the reader edits, and a
  signing key and timestamp authority the program must own. A program that signs uses a signing
  service.
- **Untagged output.** It saves a few percent of the file and gives up the document's accessibility.
