`nvs fmt` normalizes a mis-cased reserved spelling to its lower-case form only where that mis-cased
spelling has no other legal meaning. Two qualify, and each already carries the diagnostic that names the
fix: a duration literal's unit, `5Min` → `5min` (`rule:types/duration-literal`), and the open tag,
`<?NVS` → `<?nvs` (`rule:classes/reserved-spellings-are-lower-case`).

The criterion is what generalizes, not the list. A keyword never qualifies: `IF` and `ECHO` are legal
`PascalCase` class names under `rule:core-api/casing-checks-the-leading-character`, so nothing lexical
separates a mis-typed keyword from a deliberate class reference, and a formatter that rewrote one would be
the only place in the toolchain that guesses. An identifier never qualifies for a different reason: fixing
its case is a *rename*, which must reach every use site across the workspace, and `nvs fmt` is a
single-file walk — that rename is an editor's workspace-wide code action. A duration unit and an open tag
can be nothing else, which is why they and only they are here. Normalizing PHP's case-insensitive
reserved words is the converter's job, where the input is known to be PHP.
