Reads what a piece of terminal text says, as a plain string, with the styling left out.

A `Core\Cli\Text` is made of runs: a piece of text and the style that piece wears. `text()` gives
you those pieces joined, as a `string`, and no colour and no attribute comes with them. A sum of
several pieces reads back as their text in order, and an empty one reads back as `""`.

You want this whenever something other than the terminal needs the same line. A log file takes the
string, a database column takes it, and `Core\Cli::displayWidth` takes it when you count the
columns a line will fill.

Every control character was replaced by a symbol you can see when the text was made, so what you
read back holds none. You can put it straight into `Core\Cli\Text::plain` or
`Core\Cli\Text::styled` again.

**The examples below** read one line back, write the same line to the screen in colour and to a log
without it, and line up a column of coloured states.
