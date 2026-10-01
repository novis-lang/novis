`nvs doc <entry>` writes one Markdown page per class from the JSON `rule:tooling/meta-json-takes-a-program`
emits. It ships in the binary because a user's project does not have this repository's `bun nv reference`,
and it is deliberately the least interesting part of the design: a renderer with no source of truth of its
own (`rule:tooling/one-json-several-renderers`), so replacing it later costs nothing.

What it adds to the document is reading, never deciding. A page names the class's parent and interfaces
from the document's `extends` and `implements`, and a method with no doc comment of its own shows the one
it inherits by following those same keys to the nearest documented method of the same name — the comment
`rule:tooling/strict-docs` accepts for it. A name with a page in the same run becomes a link, and one
without stays code.

It renders text the lexer has already accepted, so it needs no bidi check of its own
(`rule:security/bidi-boundaries`). This repository does not itself need it — the one-file reference and the
website already cover every in-tree consumer — and it exists for a user's own project and for the package
ecosystem that does not exist yet, which is why it is kept cheap to replace.
