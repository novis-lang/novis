A comment is a note for whoever reads the file next, and the program takes no notice of it.

There are three ways to write one. `//` and `#` each start a note that runs to the end of the line,
so either can sit on a line of its own or follow code you want to explain. `/* … */` covers any
span, from the middle of one line to a whole paragraph. A note meant for the people who *call* your
code gets a third slash instead: `///` written above a class or a method is a doc comment, and your
editor shows it to whoever types that name.

**Good to know:** `#[` is not a comment. It opens an attribute, so a note that starts with a square
bracket has to use `//`. And a block comment ends at the first `*/`, which means one cannot hold
another.
