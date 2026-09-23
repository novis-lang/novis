- **A `Core` member whose return type changes is a handful of edits in its module and then a corpus
  pass, and the corpus pass is the bigger half.** Most failing cases want only an `as string` at
  each call site; the rest pin a *refusal* that moved one member along, so their `--EXPECT--`
  becomes `cannot convert `bytes` to `string`: not well-formed UTF-8 at byte N` and their titles
  must say so. Write the mechanical half as a script under `.agent-tmp/` handed to Python by path,
  never by `sed`. [until: reviewed 2026-09-06]
