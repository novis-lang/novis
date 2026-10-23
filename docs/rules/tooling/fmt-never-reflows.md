`nvs fmt` never breaks a line because it is long and never joins lines onto one. Whether a list, a
`->` call chain or an operator chain spans one line or several is the author's choice, made by writing
a line break at that construct's own level, and `nvs fmt` keeps that choice exactly. A long one-line
call is never split; a hand-wrapped call is never collapsed onto one line.

What follows from the choice is `nvs fmt`'s: a broken construct gets one fixed layout, one part per
line (`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`,
`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`,
`rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line`). Every other line break inside an
expression stays where its author wrote it, and the line after it keeps the indentation its author
wrote.

There is deliberately no line-length rule anywhere, soft or hard. With no width decision to make, a
width limit would be advisory prose with nothing in the tool to enforce it, and the editor's own ruler
already shows a column.

A width-fitting printer breaks at places nobody would choose and packs as much onto a line as fits, so
its output is often less readable than its input. Leaving every byte of an expression as written gives
the author no help either: a list wrapped two items to a line, or closed on its last item's line,
would stay exactly as typed. The fixed layout of a broken construct is the help, and the author's line
break is the only decision it needs.

The cost is stated and accepted: the formatter cannot decide by itself that a line is too long, and a
human still chooses when a construct is long enough to break, with one keystroke or with
`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`. What that buys is a formatter that walks the
existing parse tree rather than a width-fitting printer this project has no other user of, never
breaks a line at a place nobody would choose, and stays byte-for-byte deterministic as the parser
evolves (`rule:tooling/fmt-is-idempotent`).

**Declined:**
- **A width-fitting printer, as Prettier, php-cs-fixer and rustfmt are.** The tool chooses every
  break, at places nobody would choose.
- **A trailing comma as the signal to keep a list broken, as Black and dart format use.** It is a
  second, less visible way to say what a line break already says.
- **A line-length limit that only reports.** It is a diagnostic about layout, which
  `rule:tooling/fmt-is-never-a-diagnostic` excludes, and the editor's ruler already shows a column.
