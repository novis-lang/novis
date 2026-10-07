Returns the size in pixels that a text needs, without drawing it.

`Image::measureText` takes the text, a `Font` and the options `size`, `maxWidth` and `align`. These
are the same options that `text` takes. The result has a `width` and a `height` in pixels. `text`
draws the same text in a box of exactly this size.

`size` is the font size in pixels. One line is as high as the font's ascent plus its descent at that
size. A line feed starts a new line. With `maxWidth`, lines also break at spaces, and each extra line
adds to the `height`. A word wider than `maxWidth` is not broken, so the result can be wider. Only
Latin text is supported.

Text with nothing to draw throws a `LogicError`. A font file with broken tables throws a
`ParseError`.

**Good to know:** use it to make an image exactly as big as its text, or to choose a font size that
fits a space.

**The examples below** make an image the size of a label, count the lines of a wrapped text, and find
the biggest font size for a banner.
