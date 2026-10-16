Gives a colour from a red, a green and a blue value, which is how a brand colour is written.

Each of the three values is a level from `0` to `255`, so `Core\Cli\Color::rgb` reaches about
sixteen million colours. `Core\Cli\Color::rgb(30, 144, 255)` is a strong blue, and
`Core\Cli\Color::rgb(0, 0, 0)` is black. A value above `255` throws a `RuntimeError`, and the error
says which of the three it read.

You pass the colour to `Core\Cli\Style::of`, and the style to `Core\Cli\Text::styled`. A terminal
that shows every colour writes the colour exactly. A terminal with 256 colours, or with only
sixteen, writes the nearest colour it has, and output that goes to a file or to another program is
written without any colour at all. That is why a program can use its brand colours everywhere and
still be readable.

**The examples below** write one line in a brand colour, move a colour from red to green as a
number grows, and keep a whole brand's colours in one place.
