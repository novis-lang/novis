Returns the text with every control character replaced by a visible one that does nothing.

A control character is not a letter. It is an order to the terminal: move the cursor, clear the
screen, change the colour, rename the window. Text that came from a person or from a file can carry
such orders. `Core\Cli::escape` replaces each one with a single character that stands for it and is
safe to show. The escape character becomes `␛`, a carriage return becomes `␍`, the delete character
becomes `␡`. A tab and a newline lay text out rather than give orders, so they pass through.

`echo` already does this at the terminal. You need `Core\Cli::escape` when your program wants that
safe text as a value: to keep it, to measure it, or to put it in a report.

**Good to know:** the result is a plain `string`. Tainted text comes back untainted, because nothing
is left in it for a terminal to act on.

**The examples below** show an escape sequence as text, show what passes through, and hold what a
person typed.
