Checks whether an attachment is shown inside the HTML body of the mail. It returns `true` for an
inline image and `false` for a file to download.

`inlineFile` and `inlineBytes` make inline attachments. `fromFile` and `fromBytes` make files to
download. Only an inline attachment has a `src()`. On any other attachment, `src()` throws a
`RuntimeError`, so check `isInline()` first when a list has both kinds.

In a test, `Core\Test::sentMail()[0]->attachments()` returns the attachments of the first mail
that was sent, in the order they were given. `isInline()` then checks that an image is shown
inside the mail and is not a file to download.

**The examples below** list which attachments are inline, add an `<img>` tag only for an inline
image, and test that a logo is inline.
