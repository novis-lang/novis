A cursor inside a list, a `->` call chain, a binary operator chain, a ternary or the parentheses of a
control header is offered **Put on separate lines** when the innermost such construct around it is on
one line, and **Join onto one line** when it is broken. The construct and what breaks it are the layout
rules':
`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`,
`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line`,
`rule:tooling/fmt-every-binary-operator-breaks-operator-first`,
`rule:tooling/fmt-a-broken-ternary-has-a-line-per-branch` and
`rule:tooling/fmt-every-control-header-breaks-like-a-condition`.

Each action writes the layout `nvs fmt` gives the construct once the break is added or removed, so
formatting after it changes nothing more. A split puts every operand of a `.` chain on a line of its
own; `nvs fmt` keeps those lines, as it keeps any a `.` chain's author wrote. A join of a condition or
a header joins every broken chain inside it too, because any one of them breaks the header. Only line
breaks and the whitespace around them change, together with the trailing comma the layout implies
(`rule:tooling/fmt-trailing-commas`), so nothing the program does changes. Join is not offered on a
construct that holds a line comment, because joining would turn the code after the comment into
comment text.

The server computes both from the tree under the cursor alone, with no type or module question. They
are filed under `refactor.rewrite`, and are never under `source.fixAll.nvs` or `quickfix`: whether a
construct is broken is the author's choice, never something wrong.
