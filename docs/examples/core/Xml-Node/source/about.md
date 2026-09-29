Returns an XML node and everything inside it as XML text.

Use it when you read a document with `Core\Xml::parse` and want to write the whole document, or one
part of it, back out. Every element gets an end tag, so `<e/>` comes back as `<e></e>`. The
characters `&`, `<` and `>` in text are escaped, so text never turns into markup. The result has no
`<?xml ...?>` declaration at the start. It is `tainted`, because it came from outside the program.

**Good to know:** `Core\Html::parse` accepts some comments and names that XML cannot write. For such
a tree, `source` throws a `LogicError`.

**The examples below** write a document and one element back out, catch the error for a tree XML
cannot write, and copy some products out of a catalog into a new document.
