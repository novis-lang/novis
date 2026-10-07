Returns a short text that a web page can draw as a blurred preview of an image.

`Image::placeholder` reads an encoded image and returns a string of about 30 characters. You send
this string with the page. A small script in the browser draws it as a blurred picture while the
real image loads. The average colour of the preview is the average colour of the image.

The `PlaceholderKind` you pass sets the format of the text: `BlurHash` or `ThumbHash`. Use the one
your front-end script reads. Bytes that are not an image throw a `ParseError`.

**Good to know:** make the placeholder once, when the image is uploaded, and save it with the
image. The text is short, so it fits in a database column or a JSON response.

**The examples below** show a placeholder in a JSON response, a placeholder for every product photo,
and the error for bytes that are not an image.
