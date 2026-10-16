The line where this part of the source starts, counted from 1.

This is the number a person reads down the side of an editor, so a report that gives a file and a
line is one they can jump straight to. Use `column` with it when you want the exact place along the
line, and `offset` when you need the position in bytes.

The line is where the part *starts*. A class that runs from line 8 to line 30 has the line 8, and
every method inside it has its own. To find where a part ends, take the largest line among its
`nodes`.

**Good to know:** a newline inside a text value counts. Source that holds a value written over
three lines puts the next statement three lines further down, which is what an editor shows.

**The examples below** show how to name the place of each part of a program, how many lines a class
covers, and a check for lines holding more than one statement.
