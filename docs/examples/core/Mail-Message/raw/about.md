Returns a received message exactly as `Core\Mail\Message::parse` read it: every header and the
whole body, as `bytes`.

Nothing is decoded and nothing is changed. An encoded subject stays encoded, and every line ending
stays as it was. Use `raw()` to keep a copy of a message, for example as an `.eml` file that every
mail program can open, or to forward the whole message as an attached file.

The sender wrote every byte, so `raw()` returns `tainted bytes`. Tainted means that the value comes
from outside your program. To read one part of the message, use the other methods, such as
`subject()`, `text()` or `attachments()`.

**The examples below** show how to save a message as a file, that the bytes are not changed, and
how to forward a whole message.
