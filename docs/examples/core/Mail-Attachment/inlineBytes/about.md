Returns an inline attachment with the file name and the image you give. A mail program shows an
inline image inside the HTML body.

Use it when your program has the image's bytes and no file, such as a photo from a database. Write
the attachment's `src()` in the `src` of an `<img>` tag, and pass the attachment in the
`attachments` option of `Core\Mail::send`. The content can be a `string` or `bytes`.

The type, such as `image/png`, is detected from the first bytes. With the `type` option, you
set it yourself. The file name is checked as `fromBytes` checks it: a control character, such as a line
break, throws a `RuntimeError`.

`send` checks that the HTML and the inline images match. An inline image that the HTML does not
use throws a `RuntimeError`, and so does a `cid:` URL with no inline image for it.

**The examples below** show a photo from a database, one image for each product, and the error for
a `cid:` URL with no image.
