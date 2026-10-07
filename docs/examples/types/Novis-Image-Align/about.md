How the lines of a text are placed next to each other.

You pass an `Align` to `Image::text` or `Image::measureText` as `align`. It matters when a text has
more than one line, for example when `maxWidth` wraps it. Each line is placed inside a box as wide as
the longest line.

- `Left` starts every line at the left side of the box. This is the default.
- `Center` puts every line in the middle of the box.
- `Right` ends every line at the right side of the box.

`align` does not change the size of the box, so `measureText` returns the same size for all three.

The example uses a very small font that is stored in the `fonts` folder next to it.
