Returns a short text that a web page can draw as a blurred preview of an image.

`Codec::placeholder` takes an encoded image and a `PlaceholderKind`. It returns the placeholder as a
string. A `BlurHash` is 28 characters. A `ThumbHash` is base64 text and also keeps transparency. The
average colour of the preview is the average colour of the image.

`Codec::placeholder` throws a `ParseError` when the bytes are not an image in a format Novis reads.

**Good to know:** most programs do not call `Codec::placeholder` directly. `Image::placeholder` calls
it with the same arguments.
