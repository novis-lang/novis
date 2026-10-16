Gives a colour from the terminal's palette by its number.

A terminal that shows colour has a palette of 256 entries, numbered `0` to `255`.
`Core\Cli\Color::index` returns the colour at one of those numbers. Entries `0` to `15` are the
sixteen basic colours, which also have names on this class, such as `Core\Cli\Color::RED`. Entries
`16` to `231` are mixed colours, and entries `232` to `255` are greys from nearly black to nearly
white. A number above `255` throws a `RuntimeError`.

You pass the colour to `Core\Cli\Style::of`, and the style to `Core\Cli\Text::styled`. A terminal
with only sixteen colours shows the nearest one it has, and output that goes to a file or to
another program is written without any colour at all.

**The examples below** colour two lines by number, show that entry `1` and `Core\Cli\Color::RED`
are the same colour, and give every state of a build report its own colour.
