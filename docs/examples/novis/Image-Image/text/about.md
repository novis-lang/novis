Draws a text on the image with a font that your program gives it.

`text` takes the text, a `Font` and options. `size` is the font size in pixels, and `color` is the
colour of the text. Both are needed. `x` and `y` place the top-left corner of the text. Without them,
`gravity` places the text, and the default is `Gravity::Center`.

A line feed starts a new line. With `maxWidth`, lines also break at spaces. `align` sets how the
lines line up, and the default is `Align::Left`. `Image::measureText` returns the size of the text.

There are no system fonts. Make a `Font` with `Font::fromBytes`. Only Latin text is supported. Text
with nothing to draw throws a `LogicError`, and a font file with broken tables throws a
`ParseError`. Both happen when `encode` or `raw` runs.

**The examples below** draw a label on a badge, place a letter at `x` and `y`, and write a caption
at the bottom of a photo.
