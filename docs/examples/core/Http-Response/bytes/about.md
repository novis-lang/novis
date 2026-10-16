Returns the body of a reply exactly as it arrived, as `bytes`.

Use `bytes()` for a reply that is not text, for example an image, an archive or a PDF file. It
returns every byte of the body and checks nothing. `text()` returns the same body as a `string`, but
it throws a `RuntimeError` when the body is not valid UTF-8 text. You can call `bytes()` more than
once, and each call returns the same body.

The body comes from another server, so the result is `tainted` until your program checks it.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read a small image, show a body that `text()` cannot read, and check a
downloaded profile picture before it is used.
