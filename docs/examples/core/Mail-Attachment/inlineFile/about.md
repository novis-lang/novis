Reads an image from the disk and returns it as an inline attachment. A mail program shows an
inline image inside the HTML body.

Write the attachment's `src()` in the `src` of an `<img>` tag, and pass the attachment in the
`attachments` option of `Core\Mail::send`. The image is sent with the mail. The mail program does
not load anything from the internet to show it.

The file is read when `inlineFile` runs. A missing file throws an `IOError` on that line. A file
that the `fs.read` capability does not cover throws a `RuntimeError`. A relative path starts at the
folder of the program file that contains it.

`send` checks that the HTML and the inline images match. An inline image that the HTML does not
use throws a `RuntimeError`, and nothing is sent. So does a `cid:` URL in the HTML with no inline
image for it.

**The examples below** send a welcome mail with a logo, show the error for an image the HTML does
not use, and test that the logo is inside the mail.
