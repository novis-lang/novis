Creates a style for text in a terminal: a colour, a background colour and attributes such as bold.

`Core\Cli\Style::of` takes the parts by name and returns a `Core\Cli\Style`. The five attributes
are bold, dim, italic, underline and strikethrough. Every part is optional. Without a colour, the
terminal uses its own colour. An attribute that you do not name is off.

A style alone prints nothing. You give it to `Core\Cli\Text::styled` together with the text. A
style is a value, so you can create it once, keep it in a variable and use it for many lines. You
never write an escape sequence yourself.

**Good to know:** the result depends on where the output goes. A terminal with fewer colours shows
the nearest colour it has. Output that is not a terminal, such as a log file, gets the text with no
styling.

**The examples below** colour one message, put several attributes on one heading, and build one
style for each kind of line that a tool prints.
