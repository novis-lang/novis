Whether an expression, a call-argument list, an array literal, an anonymous object, a `match` arm list or an
enum-case list spans one line or several is the author's choice, and `nvs fmt` preserves it exactly. It
normalizes what surrounds that choice — the indentation of continuation lines, spacing, and brace
placement — and nothing inside it. A hand-wrapped multi-line call is never collapsed onto one line; a long
one-line call is never split.

There is deliberately no line-length rule anywhere, soft or hard. With no reflow decision to make, a width
limit would be advisory prose with nothing in the tool to enforce it.

The cost is stated and accepted: the formatter cannot repair a badly wrapped call by itself, and a human
still decides when an expression is long enough to wrap. What that buys is a formatter that is a
whitespace, brace and order normalizer walking the existing parse tree rather than a width-fitting doc
printer this project has no other user of — and one that stays trivially byte-for-byte deterministic as the
parser evolves (`rule:tooling/fmt-is-idempotent`).
