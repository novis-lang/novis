Says how much colour a terminal can show, so a program can pick a palette that will actually arrive.

Terminals differ. A log collector or a pipe shows no colour at all, an old console shows the sixteen
ANSI colours, most terminals today show a 256-entry palette, and a modern one shows all 24-bit
colours. `Core\Cli\ColorDepth` has one case for each of those — `None`, `Ansi16`, `Ansi256` and
`TrueColor` — and `Core\Cli::colorDepth()` answers with the one this run has, after reading `NO_COLOR`,
`TERM` and the rest.

Most programs never ask. You write styled text and Novis drops or downgrades what the terminal cannot
show. Ask when the *content* changes with the answer: how many shades a chart uses, or whether to
draw a picture at all.

**Good to know:** the cases are listed from least colour to most, so *at least this much* is a
comparison of their numbers — `($have as int) >= ($want as int)`. A case is a name, not a number, so
the conversion is written out.
