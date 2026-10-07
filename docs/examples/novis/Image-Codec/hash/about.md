Returns a short hash of an encoded image. Two similar images have hashes that differ in few bits.

`Codec::hash` takes an encoded image and a `HashKind`. It returns the hash as bytes. Each kind has
its own fixed length: a `Perceptual` hash is 8 bytes, a `Difference` hash is 16 bytes and an
`Average` hash is 32 bytes. A resized copy of an image has a hash close to the hash of the original.

`Codec::hash` throws a `ParseError` when the bytes are not an image in a format Novis reads.

**Good to know:** most programs do not call `Codec::hash` directly. `Image::hash` calls it, and
`Image::hashDistance` counts the bits that differ between two hashes.
