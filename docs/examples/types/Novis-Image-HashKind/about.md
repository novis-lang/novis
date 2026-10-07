The kind of hash `Image::hash` returns.

A hash is a short fingerprint of what an image looks like. Two images that look the same have the
same hash, or one with only a few different bits, even when their size or file format is different.
`Image::hashDistance` counts the bits that differ between two hashes of the same kind.

- `Perceptual` is 8 bytes. It looks at the large shapes of the image.
- `Difference` is 16 bytes. It looks at where the image gets brighter or darker.
- `Average` is 32 bytes. It looks at which parts are brighter than the image as a whole.

Each kind has its own length. `hashDistance` throws a `LogicError` when you compare two hashes of
different kinds.
