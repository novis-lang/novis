Splits a long text into lines of a fixed width.

`Core\Str::wrap` takes a text and a width, and returns the text with line breaks in it. It breaks a
line at a space, and the break replaces that space. No line is longer than the width, unless one
word alone is longer. The width counts characters, so `"é"` counts as one, the same as `"e"`.

The break is `"\n"` unless you give another one with `breakWith`, such as `"<br>"`. Where the text
already contains the break, a new line starts there.

A word that is longer than the width stays whole on a line of its own. With `cutLongWords: true`,
the word is cut into pieces of the width instead.
