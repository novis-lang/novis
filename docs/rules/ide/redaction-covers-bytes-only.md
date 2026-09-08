A range is answered when it is a string, heredoc or `bytes` literal token whose static type carries
`secret`, or an interpolation slot inside one whose interpolated expression does. Nothing else.

**Never an identifier** — a variable or property name is a name, not a secret, and concealing it hides
no bytes while making the file unreadable for the developer whose editor it is. **Never a type
annotation** — `secret string` is the declaration doing its job, and it is how a reader knows the
concealment below it is deliberate rather than a rendering fault. **Never a whole line, statement or
block**, which line-granular folding structurally cannot express anyway.

The concealment is a decoration over the range with the character cells kept, so the cursor, the
selection and every edit still address the real text. It is a rendering, not an edit: the buffer is
the file on disk, byte for byte.

**Not on disk.** There is no language server in the tree.
