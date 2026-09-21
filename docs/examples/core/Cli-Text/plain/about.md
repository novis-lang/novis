Makes terminal text out of a string, with every control character replaced by a visible symbol.

A terminal does not only show text. It also reads commands out of it: a few bytes can clear the
screen, move the cursor, change the window title or hide what comes next. Any string your program
did not write itself can contain them, such as a file name, a row from a database or something a
user typed.

`Core\Cli\Text::plain` takes a string and gives you a `Core\Cli\Text`. Every control character in it
is replaced by a symbol you can see, so an escape character arrives on screen as `␛` and is never
run. What you get back is a value you can print with `echo`, join to other terminal text with `+`,
use as a row of a live region, and read back as a string with `text()`.

`Core\Cli\Text::styled` is the same thing with a colour. The two are the only way to make terminal
text, and neither can produce a command the terminal runs.

**The examples below** print one piece of text, show what happens to an escape sequence, and build
a line out of plain and coloured parts.
