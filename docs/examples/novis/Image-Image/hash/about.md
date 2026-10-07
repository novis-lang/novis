Returns a short hash of an image, so you can find copies of the same picture.

`Image::hash` reads an encoded image, such as a PNG or JPEG file, and returns a few bytes. Two images
that look the same give hashes that are almost equal, even when one is smaller or saved in another
format. You compare two hashes with `Image::hashDistance`.

The `HashKind` you pass sets how the hash is made and how long it is: 8 bytes for `Perceptual`, 16
bytes for `Difference` and 32 bytes for `Average`. Bytes that are not an image throw a `ParseError`.

**Good to know:** a hash is small, so you can save it in a database next to each image. You then
compare a new upload with the saved hashes and never decode the old images again.

**The examples below** show the length of each kind, saving a hash as text, and the error for bytes
that are not an image.
