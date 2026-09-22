A literal is a value written straight into the program: a number, a piece of text, `true`, `false` or
`null`.

Numbers may be written the way they read best. Underscores between the digits are ignored, and a
number can be given in base sixteen, base two or base eight. A leading zero means nothing special, so
`017` is seventeen.

Text comes in two kinds of quote. Single quotes keep every character as typed. Double quotes fill in
the values of your variables and understand escapes such as `\n` for a new line. A longer block of
text is written as a heredoc, which runs over several lines. A number with a unit after it, such as
`30s` or `1h30m`, is a length of time rather than a number.

A piece of a web page is written as an `html` literal, between backticks: `` html`<p>Hi {$name}</p>` ``.
Its type is `Core\Html\Markup`, the only type a web page writes as it is. The text you typed is kept,
and every hole is escaped, so a value cannot add a tag. `{$name}` is a hole for a variable.
`<?= ... ?>` is a hole for any expression, such as a constant or a method call. A result of type
`Core\Html\Markup` is written as it is. Two fragments are joined with `+`.

**Good to know:** a literal is worked out while the program is compiled, so it costs nothing while
the program runs.

**The examples below** take these in turn: the ways to write a number, the two kinds of quote, a
message to a customer built from a template, and a page fragment with holes.
