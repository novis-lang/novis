`nvs doc <entry>` writes one Markdown page per class from the JSON `rule:tooling/meta-json-takes-a-program`
emits. It ships in the binary because a user's project does not have this repository's `bun nv reference`,
and it is deliberately the least interesting part of the design: a renderer with no source of truth of its
own (`rule:tooling/one-json-several-renderers`), so replacing it later costs nothing.

It renders text the lexer has already accepted, so it needs no bidi check of its own
(`rule:security/bidi-boundaries`). This repository does not itself need it — the one-file reference and the
website already cover every in-tree consumer — and it exists for a user's own project and for the package
ecosystem that does not exist yet, which is why it is kept cheap to replace.
