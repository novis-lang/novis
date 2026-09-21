Builds the styling a piece of terminal text wears, as a value.

A style is a colour, a background colour and five attributes: bold, dim, italic, underline and
strikethrough. `Core\Cli\Style::of` takes them by name and gives you back a `Core\Cli\Style`. Every
one is optional. Leave a colour out and the terminal keeps its own, and leave an attribute out and
it is off.

You give the style to `Core\Cli\Text::styled`, together with the text it should apply to. A style is
a plain value, so you can build it once, keep it in a variable and use it for every line of one
kind. It is never a piece of text you write yourself. There is no markup and no escape sequence to
get right, and nothing a style is given can turn into a command the terminal runs.

Novis writes what the terminal at the other end understands. A terminal with fewer colours gets the
nearest one it has, and output that is not a terminal at all, such as a log file, gets the text with
no styling.

**The examples below** colour one message, put several attributes on one heading, and build one
style for each kind of line a tool prints.
