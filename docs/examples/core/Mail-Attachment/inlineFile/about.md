Reads an image from the disk and returns it as an inline attachment. An inline image is an image
shown inside the HTML of the mail, not as a download.

Write the attachment's `src()` in the `src` of an `<img>` tag. `src()` returns a `cid:` URL, which
points at an image inside the same mail. Pass the attachment in the `attachments` option of
`Core\Mail::send`. The image is sent with the mail, so the mail program does not load it from the
internet.

The file is read when `inlineFile` runs. A missing file throws an `IOError` on that line. A file
that the `fs.read` capability does not cover throws a `RuntimeError`. A relative path starts at the
folder of the program file that contains it.

`send` checks that the HTML and the inline images match. An inline image that the HTML does not
use throws a `RuntimeError`, and nothing is sent. So does a `cid:` URL with no inline image for it.

**The examples below** send a welcome mail with a logo, show the error for an image the HTML does
not use, and test the logo.
