Makes terminal text that wears a style, out of a string and a `Core\Cli\Style`.

A style is a colour, a background colour and attributes such as bold or underline. You build one
with `Core\Cli\Style::of` and give it to `Core\Cli\Text::styled` with the text it applies to. Novis
writes the escape sequence the terminal at the other end understands, so you never write one
yourself. Where there is no terminal, such as in a log file, the styling is dropped and the text is
left.

The string is treated exactly as `Core\Cli\Text::plain` treats it. Every control character in it is
replaced by a symbol you can see, so a file name or something a user typed is safe to colour. The
result carries one escape sequence for the style, and no other one.

You print the result with `echo`, join it to other terminal text with `+`, and read the string back
without the styling with `text()`.

**The examples below** put a style on one message, colour a value that came from outside the
program, and highlight the word a search found in each line.
