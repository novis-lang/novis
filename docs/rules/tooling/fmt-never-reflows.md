`nvs fmt` never breaks a line because it is long and never joins lines onto one. Whether a list, a
`->` call chain or an operator chain spans one line or several is the author's choice, made by writing
a line break at that construct's own level, and `nvs fmt` keeps that choice exactly. A long one-line
call is never split; a hand-wrapped call is never collapsed onto one line.

What follows from the choice is `nvs fmt`'s: a broken construct gets one fixed layout, one part per
line (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`,
`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`,
`rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line`). Every other line break inside an
expression stays where its author wrote it, with the indentation of the line it continues.

There is deliberately no line-length rule anywhere, soft or hard. With no width decision to make, a
width limit would be advisory prose with nothing in the tool to enforce it, and the editor's own ruler
already shows a column.

The cost is stated and accepted: the formatter cannot decide by itself that a line is too long, and a
human still chooses when a construct is long enough to break, with one keystroke or with
`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`. What that buys is a formatter that walks the
existing parse tree rather than a width-fitting printer this project has no other user of, never
breaks a line at a place nobody would choose, and stays byte-for-byte deterministic as the parser
evolves (`rule:tooling/fmt-is-idempotent`).
