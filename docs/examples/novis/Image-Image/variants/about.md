Writes several files from one image, for example the sizes a web page needs, and decodes the input
only once.

The argument is an array of entries. Each entry may have these keys:

- `width` and `height` resize the image, as `resize` does.
- `fit` decides how the image fills the size when both `width` and `height` are given.
- `format` is the file format of this entry.
- `quality` is the quality of this entry, from 1 to 100. It needs a `format`, from the entry or from
  an earlier `format` call. Without one, `variants` throws a `LogicError`.

Every entry first runs the steps of the image, and then its own resize and format. `variants` returns
one encoded file per entry, in the same order. An empty array returns an empty array.

All entries run in one call to the image component. This is faster than one `encode` per size,
because the input is decoded only once.

**Good to know:** a common use is one call that writes every width of a responsive image.
