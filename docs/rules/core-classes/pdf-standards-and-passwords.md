**Every render is a tagged PDF**: headings, paragraphs, lists, tables with their header cells, figures
with their `alt` text and the document's language come from the HTML, with no option to turn it off.
`Document::standard` adds one of **PDF/A-2b, PDF/A-3b or PDF/UA-1**, and a document that breaks the
standard it chose throws `LogicError` naming what broke — an image with no `alt` under PDF/UA-1, a
font that cannot be embedded under PDF/A — through `krilla`'s own validation. One standard per
document, because `krilla` validates against one. PDF/A needs `Document::created`, because the
component reads no clock, and PDF/UA-1 needs a `<title>`. A missing glyph, reported and never thrown
otherwise, throws under any of the three, because each forbids the empty box drawn for it.

**`Document::password`** encrypts with **AES-256, revision 6**, and nothing weaker. Its salts and its
file key are derived from a hash of the document and the password, so equal input stays byte-equal
(`rule:core-classes/pdf-output-is-inert`). A password beside a PDF/A standard throws `LogicError`,
because PDF/A forbids encryption.

Tagging costs a structure tree the size of the element tree and buys a document a screen reader can
read; it is the default because a program that never thought about accessibility should still produce
an accessible file. AES-256 alone is offered because every reader made in the last fifteen years opens
it and the weaker revisions are broken.

- **Memory:** the structure tree, beside the layout tree inside the request's cap, and with a
  password one more copy of the finished file while it is encrypted.
- **Performance:** a password adds four runs of the standard's password hash, each 64 or more rounds
  of AES and SHA-2, and one AES pass over the file.

**On disk.** `Document::standard` and `Document::password` are in
`extensions/pdf/nvs/Document.nvs`, and a password beside a PDF/A standard is refused before anything
is laid out (`extensions/pdf/src/lib.rs`'s `first`). Tagging and the three standards are
`extensions/pdf/src/write.rs`, and the encryption is `extensions/pdf/src/encrypt.rs`, over the file
`krilla` writes because `krilla` 0.8 writes no encryption. The password is both the user and the owner
password. All of it is pinned by `crates/nvs-ext/tests/pdf_standards.rs`, which opens the encrypted
file with `hayro-syntax`.

**Declined:**
- **RC4 and AES-128 encryption, and owner permissions.** RC4 is broken, and permission flags are a
  request to the reader that any tool ignores.
- **Form filling and signatures.** Each is a second model of the file: fields the reader edits, and a
  signing key and timestamp authority the program must own. A program that signs uses a signing
  service.
- **Untagged output.** It saves a few percent of the file and gives up the document's accessibility.
