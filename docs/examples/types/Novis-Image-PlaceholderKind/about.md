The kind of placeholder `Image::placeholder` returns.

A placeholder is a short text that describes the colours of an image. A web page draws a blurred
picture from it while the real image loads. You save the placeholder next to the image, and the
browser decodes it with a small script for that format.

- `BlurHash` is 28 characters. It has no transparency.
- `ThumbHash` is base64 text. It also keeps transparency.

Choose the format that your front-end library reads.
