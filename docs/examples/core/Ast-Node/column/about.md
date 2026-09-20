The column where this part of the source starts, counted from 1.

The count is in characters, not bytes, so an accented letter or an emoji counts as one. That is what
an editor shows and what a person counts along a line. Use `line` together with it to name a place
somebody can go to and look at, and `offset` when you need the position in bytes instead.

**Good to know:** the column is the first character of that part, not of the line it sits on. A
statement indented by four spaces starts at column 5.

**The examples below** show where each part of a program begins, a line holding an accented letter,
and a report that points at the exact place with a mark under it.
