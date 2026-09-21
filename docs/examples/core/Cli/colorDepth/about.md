Says how much colour the terminal can show, so a program can leave colour out where there is none.

Terminals differ. Some show no colour at all, some show the sixteen basic colours, some show 256,
and some show every colour a screen has. Where the output goes matters too: text sent to a file or
to another program has no colour. `Core\Cli::colorDepth` returns one of four cases: `None`,
`Ansi16`, `Ansi256` or `TrueColor`. It reads the settings a person expects it to read, such as
`NO_COLOR` and `TERM`, and it gives the same answer for the whole run.

**Good to know:** most programs never ask. When you write a `Core\Cli\Text`, Novis already leaves
the colour out on a terminal that cannot show it. You ask when the choice is your program's own,
such as picking a different mark or a different drawing.

**The examples below** print what the terminal can show, pick between two ways of drawing a chart,
and write the state as a word where colour cannot carry it.
