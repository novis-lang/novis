Returns the notes of one sheet. A note is the short text that Excel shows when you point at a cell
with a red corner.

`$sheet` is the sheet name, or its position from 0. Each note has `noted`, the cell of the note,
such as `"B2"`, `note`, the text, and `author`, the name of the person who wrote it. `author` is
`null` when the note has no author.

**Good to know:** the text and the author come from the file, so they are `tainted`. A CSV file has
no notes, so the list is empty. A sheet that does not exist throws a `LogicError`. `notes` reads
xlsx, xlsm, xlsb, xls and ods files. In an ods file, a note is called a comment.

**The examples below** list the notes of a sheet, read a note without an author, and show each row
of an import together with its notes.
