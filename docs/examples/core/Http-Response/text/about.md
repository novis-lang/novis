Returns the body of a reply as a `string`.

Use `text()` for a reply that is text, for example a web page, a CSV file or a plain version
number. The body must be valid UTF-8 text. If it is not, `text()` throws a `RuntimeError`. The
message says which byte is the first one that is not text. `bytes()` returns the same body without
this check. You can call `text()` more than once, and each call returns the same text.

The body comes from another server, so the result is `tainted` until your program checks it.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read a small text file, show a reply in an old encoding that is not UTF-8,
and check an update server for a new version number.
