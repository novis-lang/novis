A fact a repository tool reads — a goal, the chain, a rule's structure, a decision's fields, a gap,
a playbook bullet, a plan field — is one JSON file under `data/`, holding one entity. Its type is
declared once in `tools/nv/schema/`, and that declaration is the static type, the runtime check and
the JSON Schema together. `tools/nv/lib/store.ts` is the only code that writes a record: keys in
schema order, two-space indent, LF and one trailing newline. So one value has one text, and git merges
two edits to one record line by line.

An id is a slug or a number, and it never changes. A reference is an id, and `bun nv check` fails on
one that names no record.

Prose stays under `docs/` at its own path — a rule fragment, a decision record, a goal's prose, a
reference chapter — so every citation into it still resolves. Where a prose file repeats a field a
record holds, the record is the authority and a check compares the copy to it.
