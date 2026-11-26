Returns the URL of an inline image, such as `cid:3f2a…@inline`. An inline image is an image shown
inside the HTML of the mail. A `cid:` URL points at an image inside the same mail. Write it in the
`src` of an `<img>` tag, and the mail program shows the image at that place.

Only an attachment made with `inlineFile` or `inlineBytes` has a `src()`. On an attachment made
with `fromFile` or `fromBytes`, the call throws a `RuntimeError`.

The URL is random, and it is made once, when the attachment is made. Every call returns the same
URL, and two images never have the same one. So you can build the HTML before you send the mail.

`Core\Mail::send` checks each `cid:` URL in the HTML. The URL must be the `src()` of an inline
attachment of the same mail. A URL from another mail, or one you typed, throws a `RuntimeError`,
and nothing is sent.

**The examples below** show what a URL looks like, the error for a file to download, and a test
that checks the HTML uses each image.
