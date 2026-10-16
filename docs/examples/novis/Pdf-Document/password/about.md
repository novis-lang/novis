Encrypts the PDF file, so a reader needs the password to open it.

The encryption is AES-256, which every current PDF viewer can open. The same password opens the file
and allows all changes to it. The same document with the same password always gives the same bytes.

A PDF/A file cannot have a password, so `password` with a PDF/A standard makes `render` throw a
`LogicError`. `Reader` and `Pages` cannot open an encrypted file. They throw a `ParseError`.

**Good to know:** a password protects a file that travels, such as a statement sent by email. Send
the password another way, never in the same email.

**The examples below** encrypt a statement, give each customer their own password, and show the
error for a PDF/A file.
