The error for input that did not match a format the program declared: a document that is not the
document it was announced as, a field that will not convert to the type it is read into, a value
nested deeper than the reader was allowed to go. It sits on the `RuntimeError` side of the tree,
because being handed a bad document is the world's doing rather than the program's.

What makes it different from every other error is the list it carries. Besides the message and the
place it was thrown from, a `ParseError` has `issues`: one entry per thing that was wrong, each
naming the field it was found at and what was wrong there. A form with three bad fields is one
error reporting all three, so a reply can tell a person about every one of them instead of one per
attempt. A document that is simply broken is one entry whose field name is empty — nothing in
particular was at fault, the whole thing was.

**The examples below** show a form whose every bad field is reported at once, a document that is not
JSON at all, and a reply naming each field a signup got wrong.
