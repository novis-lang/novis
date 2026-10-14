Adds the note `$text` to the cell `$cell`, and returns the sheet.

Excel shows a small red corner on a cell with a note, and shows the note when the mouse is over the
cell. `author` is the name the note shows. Without it, the note has no author.

**Good to know:** the text of a note is written as plain text. Markup and formulas in it are not run,
so a note can show text a user typed. The cell can be in a row you already added or in a row you add
later. A CSV file has no notes, so `note` does nothing there.

**The examples below** add a note to a cell, add a note with an author, and add a note to each row
that needs to be checked.
