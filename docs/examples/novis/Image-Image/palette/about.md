Returns the main colours of an image, the most common colour first.

`Image::palette` reads an encoded image and returns an array of `Color` values. You choose how many
colours you want with `count`, which is 5 by default. The array can be shorter than `count` when the
image has fewer colours. Transparent pixels are not counted.

A `count` above 256 throws a `LogicError`. Bytes that are not an image throw a `ParseError`.

**Good to know:** use the first colour as the background of a card or a page header, so the page
matches the photo. Each `Color` has the numbers `r`, `g`, `b` and `alpha`. You use them to write
the colour in CSS.

**The examples below** show the colours of a photo with two colours, a page header in the main
colour, and the error for a `count` that is too large.
