Returns an inline attachment with the file name and the image you give. An inline image is an
image shown inside the HTML of the mail, not as a download.

Use it for an image that is not in a file, such as a photo from a database. Write the attachment's
`src()` in the `src` of an `<img>` tag. `src()` returns a `cid:` URL, which points at an image
inside the same mail. Pass the attachment in the `attachments` option of `Core\Mail::send`.

The type is a media type, such as `image/png`. It says what kind of file it is. `inlineBytes`
detects it from the first bytes. With the `type` option, you set it yourself. A file name with a
control character, such as a line break, throws a `RuntimeError`.

`send` checks that the HTML and the inline images match. An inline image that the HTML does not
use throws a `RuntimeError`, and so does a `cid:` URL with no inline image for it.

**The examples below** show a photo from a database, one image for each product, and the error for
a `cid:` URL with no image.
