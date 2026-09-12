`nvs fmt` normalizes a mis-cased reserved spelling to its lower-case form only where that mis-cased
spelling has no other legal meaning. Two qualify, and each already carries the diagnostic that names the
fix: a duration literal's unit, `5Min` → `5min` (`rule:types/duration-literal`), and the open tag,
`<?NVS` → `<?nvs` (`rule:classes/reserved-spellings-are-lower-case`).

Both spellings are errors, and that is how `nvs fmt` reaches them rather than what stops it: the
formatter refuses a file for an error it does not itself rewrite, not for reporting one at all. A
mis-cased reserved spelling leaves a whole tree behind it — the lexer read the tag or the duration
literal it was handed, and `E_RESERVED_SPELLING_CASE`'s primary span is exactly the bytes to
lower-case — so the file is formatted and the spelling goes out in its one form. The alternative,
leaving the rewrite to an editor's code action and having `nvs fmt` refuse the file, would make this
rule name two rewrites nothing performs. What the formatter does not promise is that the result
compiles: a literal that is mis-cased *and* out of order comes back lower-cased and still refused,
by the diagnostic that was always its own.

The criterion is what generalizes, not the list. A keyword never qualifies: `IF` and `ECHO` are legal
`PascalCase` class names under `rule:core-api/casing-checks-the-leading-character`, so nothing lexical
separates a mis-typed keyword from a deliberate class reference, and a formatter that rewrote one would be
the only place in the toolchain that guesses. An identifier never qualifies for a different reason: fixing
its case is a *rename*, which must reach every use site across the workspace, and `nvs fmt` is a
single-file walk — that rename is an editor's workspace-wide code action. A duration unit and an open tag
can be nothing else, which is why they and only they are here. Normalizing PHP's case-insensitive
reserved words is the converter's job, where the input is known to be PHP.
