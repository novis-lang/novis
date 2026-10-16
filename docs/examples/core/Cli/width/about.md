The terminal's width, in columns.

A column is one character cell on screen. `Core\Cli::width` tells you how many of them fit on a
line, so your program can wrap a paragraph, shorten a long path, or draw a bar that reaches the edge
and no further.

The width is read once, when the program starts. Two calls in one run always agree, so a line you
measured and a line you printed cannot disagree about how wide the screen is.

**Good to know:** the result is never `0`, so you can subtract a margin from it without checking
first. When no standard stream is a terminal the result is `80`, which is the width these examples
print for.

**The examples below** print the width, draw a line across the screen, and shorten a long path so
the line does not wrap.
