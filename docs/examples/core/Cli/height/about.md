The terminal's height, in rows.

A row is one line on screen. `Core\Cli::height` tells you how many lines the window shows at once, so
your program can print one page at a time, keep a heading in view, or stop a long list from scrolling
away before anybody reads it.

The height is read once, when the program starts, in the same step as the width. Two calls in one run
always agree.

**Good to know:** the result is never `0`, so you can take a heading and a footer off it without
checking first. When no standard stream is a terminal the result is `24`, which is the height these
examples print for.

**The examples below** print the height, leave room for a heading and a footer, and show only as many
rows of a report as fit on the screen.
