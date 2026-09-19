A literal is a value written straight into the program: a number, a piece of text, `true`, `false` or
`null`.

Numbers may be written the way they read best. Underscores between the digits are ignored, and a
number can be given in base sixteen, base two or base eight. A leading zero means nothing special, so
`017` is seventeen.

Text comes in two kinds of quote. Single quotes keep every character as typed. Double quotes fill in
the values of your variables and understand escapes such as `\n` for a new line. A longer block of
text is written as a heredoc, which runs over several lines. A number with a unit after it, such as
`30s` or `1h30m`, is a length of time rather than a number.

**Good to know:** a literal is worked out while the program is compiled, so it costs nothing while
the program runs.

**The examples below** take these in turn: the ways to write a number, the two kinds of quote, and a
message to a customer built from a template.
