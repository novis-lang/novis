A cursor inside a list, a `->` call chain or an `&&`, `||`, `??` or `.` chain is offered **Put on
separate lines** when the innermost such construct around it is on one line, and **Join onto one
line** when it is broken. The construct and what breaks it are the three layout rules':
`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`,
`rule:tooling/fmt-a-broken-call-chain-is-one-call-per-line` and
`rule:tooling/fmt-a-broken-operator-chain-is-one-operand-per-line`.

Each action writes the layout `nvs fmt` gives the construct once the break is added or removed, so
formatting after it changes nothing more. Only line breaks and the whitespace around them change,
together with the trailing comma the layout implies (`rule:tooling/fmt-trailing-commas`), so nothing
the program does changes. Join is not offered on a construct that holds a line comment, because
joining would turn the code after the comment into comment text.

The server computes both from the tree under the cursor alone, with no type or module question. They
are filed under `refactor.rewrite`, and are never under `source.fixAll.nvs` or `quickfix`: whether a
construct is broken is the author's choice, never something wrong.
