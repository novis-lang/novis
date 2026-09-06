`secret` is a second, independent compile-time qualifier on `string` and `bytes`. It and `tainted` are
independent bits rather than a combined enum: a value can be plain, `tainted`, `secret`, or `secret
tainted`. When both are spelled together, **`secret` comes first** — the only accepted order, and
`tainted secret string` is a diagnostic naming the required order rather than a second valid spelling
of the same type.

Like `tainted` it is a reserved keyword and a production in the type grammar, checked once and erased
before codegen, so a `secret` value costs nothing at run time
(`rule:security/tainted-qualifier`).

Two orthogonal questions — trust and confidentiality — get their own checked axis instead of being
conflated into one. They compose for the case that matters most in practice, a submitted password,
without needing a third combined concept.
