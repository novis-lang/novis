The place where this part of the source starts, counted in bytes from the beginning of the text.

The first byte of the source is 0. This is what you use to cut a part of the program out of the text
you gave to `Core\Ast::parse`. Take the offset of one part and the offset of the part after it, and
the bytes between them are the first part's own source.

Bytes are not characters. An accented letter takes two bytes and an emoji takes four, so a line
holding one puts every later part further along in bytes than a person counts along the line. Use
`column` for the number a person counts, and `line` with it when you want to name a place somebody
can go and look at.

**The examples below** show where each part of a program starts, how to cut the source of one part
out of the text, and how to find the part an editor's cursor is sitting in.
